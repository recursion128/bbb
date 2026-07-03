use bbb_renderer::ItemFrameMapTexture;
use bbb_world::MapItemState;

pub(crate) const MAP_SIZE: usize = 128;

// Vanilla `MapColor.MATERIAL_COLORS`, ids 0..=61; 62 and 63 fall back to `NONE`.
const MAP_MATERIAL_COLORS: [u32; 64] = [
    0, 8_368_696, 16_247_203, 13_092_807, 16_711_680, 10_526_975, 10_987_431, 31_744, 16_777_215,
    10_791_096, 9_923_917, 7_368_816, 4_210_943, 9_402_184, 16_776_437, 14_188_339, 11_685_080,
    6_724_056, 15_066_419, 8_375_321, 15_892_389, 5_000_268, 10_066_329, 5_013_401, 8_339_378,
    3_361_970, 6_704_179, 6_717_235, 10_040_115, 1_644_825, 16_445_005, 6_085_589, 4_882_687,
    55_610, 8_476_209, 7_340_544, 13_742_497, 10_441_252, 9_787_244, 7_367_818, 12_223_780,
    6_780_213, 10_505_550, 3_746_083, 8_874_850, 5_725_276, 8_014_168, 4_996_700, 4_993_571,
    5_001_770, 9_321_518, 2_430_480, 12_398_641, 9_715_553, 6_035_741, 1_474_182, 3_837_580,
    5_647_422, 1_356_933, 6_579_300, 14_200_723, 8_365_974, 0, 0,
];
const MAP_BRIGHTNESS_MODIFIERS: [u32; 4] = [180, 220, 255, 135];

pub(crate) fn map_item_texture(map: &MapItemState) -> ItemFrameMapTexture {
    let mut rgba = Vec::with_capacity(MAP_SIZE * MAP_SIZE * 4);
    for index in 0..MAP_SIZE * MAP_SIZE {
        rgba.extend_from_slice(&map_color_rgba8(
            map.colors.get(index).copied().unwrap_or(0),
        ));
    }
    ItemFrameMapTexture {
        map_id: map.id,
        rgba,
    }
}

pub(crate) fn map_color_rgba8(packed: u8) -> [u8; 4] {
    let material_id = usize::from(packed >> 2);
    let base = MAP_MATERIAL_COLORS.get(material_id).copied().unwrap_or(0);
    if base == 0 {
        return [0, 0, 0, 0];
    }
    let modifier = MAP_BRIGHTNESS_MODIFIERS[usize::from(packed & 3)];
    let r = ((base >> 16) & 0xFF) * modifier / 255;
    let g = ((base >> 8) & 0xFF) * modifier / 255;
    let b = (base & 0xFF) * modifier / 255;
    [r as u8, g as u8, b as u8, 255]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_color_rgba_matches_vanilla_map_color_scaling() {
        // Vanilla `MapColor.GRASS` is 0x7fb238. Brightness HIGH (id 2) leaves it unscaled.
        assert_eq!(map_color_rgba8((1 << 2) | 2), [0x7f, 0xb2, 0x38, 255]);
        // Brightness NORMAL (id 1) uses ARGB.scaleRGB(color, 220), i.e. integer channel * 220 / 255.
        assert_eq!(map_color_rgba8((1 << 2) | 1), [109, 153, 48, 255]);
        assert_eq!(map_color_rgba8(0), [0, 0, 0, 0]);
    }
}
