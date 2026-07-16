use bbb_renderer::ItemFrameMapTexture;
use bbb_world::MapItemState;

pub(crate) const MAP_SIZE: usize = 128;

include!("map_textures/generated_26_1.rs");

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
    fn map_color_rgba_matches_generated_full_packed_domain() {
        for packed in u8::MIN..=u8::MAX {
            let base = MAP_MATERIAL_COLORS[usize::from(packed >> 2)];
            let modifier = MAP_BRIGHTNESS_MODIFIERS[usize::from(packed & 3)];
            let expected = if base == 0 {
                [0, 0, 0, 0]
            } else {
                [
                    ((((base >> 16) & 0xFF) * modifier) / 255) as u8,
                    ((((base >> 8) & 0xFF) * modifier) / 255) as u8,
                    (((base & 0xFF) * modifier) / 255) as u8,
                    255,
                ]
            };
            assert_eq!(map_color_rgba8(packed), expected, "packed id {packed}");
        }
    }
}
