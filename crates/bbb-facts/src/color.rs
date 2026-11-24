//! Packed RGB/ARGB arithmetic shared by environment attributes and rendering.

pub const fn channel(value: u32, shift: u32) -> u32 {
    value >> shift & 0xff
}

pub const fn color(alpha: u32, red: u32, green: u32, blue: u32) -> u32 {
    (alpha & 0xff) << 24 | (red & 0xff) << 16 | (green & 0xff) << 8 | blue & 0xff
}

// Vanilla 26.1: net/minecraft/util/ARGB.java:155-160; net/minecraft/util/Mth.java:523-525.
pub fn srgb_lerp(alpha: f32, from: u32, to: u32) -> u32 {
    let lerp = |shift| {
        let from = channel(from, shift) as i32;
        let to = channel(to, shift) as i32;
        (from + (alpha * (to - from) as f32).floor() as i32) as u32
    };
    color(lerp(24), lerp(16), lerp(8), lerp(0))
}

// Vanilla 26.1: net/minecraft/util/ARGB.java:88-94.
pub fn add_rgb(left: u32, right: u32) -> u32 {
    color(
        channel(left, 24),
        (channel(left, 16) + channel(right, 16)).min(255),
        (channel(left, 8) + channel(right, 8)).min(255),
        (channel(left, 0) + channel(right, 0)).min(255),
    )
}

pub fn subtract_rgb(left: u32, right: u32) -> u32 {
    color(
        channel(left, 24),
        channel(left, 16).saturating_sub(channel(right, 16)),
        channel(left, 8).saturating_sub(channel(right, 8)),
        channel(left, 0).saturating_sub(channel(right, 0)),
    )
}

// Vanilla 26.1: net/minecraft/util/ARGB.java:80-86.
pub fn multiply(left: u32, right: u32) -> u32 {
    if left == u32::MAX {
        return right;
    }
    if right == u32::MAX {
        return left;
    }
    color(
        channel(left, 24) * channel(right, 24) / 255,
        channel(left, 16) * channel(right, 16) / 255,
        channel(left, 8) * channel(right, 8) / 255,
        channel(left, 0) * channel(right, 0) / 255,
    )
}

// Vanilla 26.1: net/minecraft/util/ARGB.java:126-129.
fn greyscale(value: u32) -> u32 {
    let gray = (channel(value, 16) as f32 * 0.3
        + channel(value, 8) as f32 * 0.59
        + channel(value, 0) as f32 * 0.11) as u32;
    color(channel(value, 24), gray, gray, gray)
}

// Vanilla 26.1: net/minecraft/util/ARGB.java:104-114.
fn scale_rgb(value: u32, scale: f32) -> u32 {
    scale_rgb_channels(value, scale, scale, scale)
}

pub fn scale_rgb_channels(value: u32, red: f32, green: f32, blue: f32) -> u32 {
    color(
        channel(value, 24),
        ((channel(value, 16) as f32 * red) as i32).clamp(0, 255) as u32,
        ((channel(value, 8) as f32 * green) as i32).clamp(0, 255) as u32,
        ((channel(value, 0) as f32 * blue) as i32).clamp(0, 255) as u32,
    )
}

// Vanilla 26.1: net/minecraft/util/ARGB.java:131-153.
pub fn alpha_blend(destination: u32, source: u32) -> u32 {
    let destination_alpha = channel(destination, 24);
    let source_alpha = channel(source, 24);
    if source_alpha == 255 {
        return source;
    }
    if source_alpha == 0 {
        return destination;
    }
    let alpha = source_alpha + destination_alpha * (255 - source_alpha) / 255;
    let blend_channel = |shift| {
        (channel(source, shift) * source_alpha
            + channel(destination, shift) * (alpha - source_alpha))
            / alpha
    };
    color(alpha, blend_channel(16), blend_channel(8), blend_channel(0))
}

// Vanilla 26.1: net/minecraft/world/attribute/modifier/ColorModifier.java:32-36.
pub fn blend_to_gray(value: u32, brightness: f32, factor: f32) -> u32 {
    srgb_lerp(factor, value, scale_rgb(greyscale(value), brightness))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn srgb_lerp_preserves_signed_channel_rounding_before_packing() {
        assert_eq!(
            srgb_lerp(2.0_f32.powi(-25), 0x80808080, 0x7f7f7f7f),
            0x7f7f7f7f
        );
        assert_eq!(srgb_lerp(-0.5, 0, 0x01010101), 0xffffffff);
    }
}
