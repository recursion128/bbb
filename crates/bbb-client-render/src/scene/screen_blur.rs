//! Vanilla `Options.menuBackgroundBlurriness`; the blur itself is the `minecraft:blur` post chain.

pub const VANILLA_DEFAULT_MENU_BACKGROUND_BLURRINESS: u8 = 5;
pub const VANILLA_MAX_MENU_BACKGROUND_BLURRINESS: u8 = 10;

pub(crate) fn sanitize_menu_background_blurriness(value: u8) -> f32 {
    f32::from(value.min(VANILLA_MAX_MENU_BACKGROUND_BLURRINESS))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_blur_radius_matches_vanilla_option_range() {
        assert_eq!(sanitize_menu_background_blurriness(0), 0.0);
        assert_eq!(sanitize_menu_background_blurriness(5), 5.0);
        assert_eq!(sanitize_menu_background_blurriness(10), 10.0);
        assert_eq!(sanitize_menu_background_blurriness(u8::MAX), 10.0);
    }
}
