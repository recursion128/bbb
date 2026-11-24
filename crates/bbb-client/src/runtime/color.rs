use super::*;

pub(super) const fn rgb24(color: i32) -> [f32; 3] {
    let rgb = color as u32;
    [
        ((rgb >> 16) & 0xff) as f32 / 255.0,
        ((rgb >> 8) & 0xff) as f32 / 255.0,
        (rgb & 0xff) as f32 / 255.0,
    ]
}

pub(super) const fn rgba32(color: i32) -> [f32; 4] {
    let argb = color as u32;
    [
        ((argb >> 16) & 0xff) as f32 / 255.0,
        ((argb >> 8) & 0xff) as f32 / 255.0,
        (argb & 0xff) as f32 / 255.0,
        ((argb >> 24) & 0xff) as f32 / 255.0,
    ]
}

pub(super) const fn argb_to_rgba8(color: i32) -> [u8; 4] {
    let argb = color as u32;
    [
        ((argb >> 16) & 0xff) as u8,
        ((argb >> 8) & 0xff) as u8,
        (argb & 0xff) as u8,
        ((argb >> 24) & 0xff) as u8,
    ]
}

pub(super) const fn argb_color(alpha: i32, red: i32, green: i32, blue: i32) -> i32 {
    packed_color::color(alpha as u32, red as u32, green as u32, blue as u32) as i32
}

pub(super) fn rgb01_to_argb(color: [f32; 3]) -> i32 {
    argb_color(
        255,
        argb_channel_from_unit(color[0]),
        argb_channel_from_unit(color[1]),
        argb_channel_from_unit(color[2]),
    )
}

pub(super) fn rgba01_to_argb(color: [f32; 4]) -> i32 {
    argb_color(
        argb_channel_from_unit(color[3]),
        argb_channel_from_unit(color[0]),
        argb_channel_from_unit(color[1]),
        argb_channel_from_unit(color[2]),
    )
}

pub(super) fn argb_channel_from_unit(value: f32) -> i32 {
    (value.clamp(0.0, 1.0) * 255.0).floor() as i32
}

pub(super) fn argb_alpha(color: i32) -> i32 {
    packed_color::channel(color as u32, 24) as i32
}

pub(super) fn argb_red(color: i32) -> i32 {
    packed_color::channel(color as u32, 16) as i32
}

pub(super) fn argb_green(color: i32) -> i32 {
    packed_color::channel(color as u32, 8) as i32
}

pub(super) fn argb_blue(color: i32) -> i32 {
    packed_color::channel(color as u32, 0) as i32
}

pub(super) fn argb_opaque(color: i32) -> i32 {
    argb_color(255, argb_red(color), argb_green(color), argb_blue(color))
}

pub(super) fn argb_srgb_lerp(alpha: f32, from: i32, to: i32) -> i32 {
    packed_color::srgb_lerp(alpha, from as u32, to as u32) as i32
}

pub(super) fn argb_multiply(lhs: i32, rhs: i32) -> i32 {
    packed_color::multiply(lhs as u32, rhs as u32) as i32
}

pub(super) fn argb_alpha_blend(destination: i32, source: i32) -> i32 {
    packed_color::alpha_blend(destination as u32, source as u32) as i32
}

pub(super) fn sample_periodic_float_keyframes(
    day_time: i64,
    keyframes: &[(i64, f32)],
    period: i64,
) -> f32 {
    sample_periodic_keyframes(day_time, keyframes, period, |alpha, from, to| {
        from + alpha * (to - from)
    })
}

pub(super) fn sample_periodic_argb_keyframes(
    day_time: i64,
    keyframes: &[(i64, i32)],
    period: i64,
) -> i32 {
    sample_periodic_keyframes(day_time, keyframes, period, argb_srgb_lerp)
}

fn sample_periodic_keyframes<T: Copy>(
    day_time: i64,
    keyframes: &[(i64, T)],
    period: i64,
    lerp: impl Fn(f32, T, T) -> T,
) -> T {
    debug_assert!(!keyframes.is_empty());
    if keyframes.len() == 1 {
        return keyframes[0].1;
    }

    let sample_tick = day_time.rem_euclid(period);
    let first = keyframes[0];
    let last = keyframes[keyframes.len() - 1];
    if sample_tick < first.0 {
        return sample_keyframe_segment(
            sample_tick,
            last.0 - period,
            last.1,
            first.0,
            first.1,
            &lerp,
        );
    }

    for window in keyframes.windows(2) {
        let from = window[0];
        let to = window[1];
        if sample_tick < to.0 {
            return sample_keyframe_segment(sample_tick, from.0, from.1, to.0, to.1, &lerp);
        }
    }

    sample_keyframe_segment(
        sample_tick,
        last.0,
        last.1,
        first.0 + period,
        first.1,
        &lerp,
    )
}

fn sample_keyframe_segment<T: Copy>(
    sample_tick: i64,
    from_tick: i64,
    from_value: T,
    to_tick: i64,
    to_value: T,
    lerp: &impl Fn(f32, T, T) -> T,
) -> T {
    if sample_tick <= from_tick {
        return from_value;
    }
    if sample_tick >= to_tick {
        return to_value;
    }
    let alpha = (sample_tick - from_tick) as f32 / (to_tick - from_tick) as f32;
    lerp(alpha, from_value, to_value)
}
