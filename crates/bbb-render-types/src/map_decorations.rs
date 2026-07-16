/// Vanilla `MapDecorationTypes` entry in built-in registry order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaMapDecorationType {
    pub type_id: i32,
    pub resource_id: &'static str,
    pub sprite_id: &'static str,
    pub render_on_frame: bool,
}

include!("map_decorations/generated_26_1.rs");

pub fn vanilla_map_decoration_types() -> impl ExactSizeIterator<Item = VanillaMapDecorationType> {
    VANILLA_MAP_DECORATION_TYPES.iter().copied()
}

pub fn vanilla_map_decoration_type(type_id: i32) -> Option<VanillaMapDecorationType> {
    VANILLA_MAP_DECORATION_TYPES
        .get(usize::try_from(type_id).ok()?)
        .copied()
}

pub fn vanilla_map_decoration_type_for_resource_id(
    resource_id: &str,
) -> Option<VanillaMapDecorationType> {
    let path = resource_id
        .strip_prefix("minecraft:")
        .unwrap_or(resource_id);
    if path.is_empty() || path.contains(':') {
        return None;
    }
    VANILLA_MAP_DECORATION_TYPES
        .iter()
        .find(|entry| entry.resource_id.strip_prefix("minecraft:") == Some(path))
        .copied()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemFrameMapDecorationTexture {
    pub sprite_id: String,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_decoration_lookup_accepts_default_namespace_and_rejects_other_namespaces() {
        assert_eq!(
            vanilla_map_decoration_type_for_resource_id("minecraft:mansion"),
            vanilla_map_decoration_type(8)
        );
        assert_eq!(
            vanilla_map_decoration_type_for_resource_id("trial_chambers"),
            vanilla_map_decoration_type(34)
        );
        assert_eq!(
            vanilla_map_decoration_type_for_resource_id("example:mansion"),
            None
        );
    }
}
