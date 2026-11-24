pub use bbb_facts::map_decorations::{
    vanilla_map_decoration_type, vanilla_map_decoration_type_for_resource_id,
    vanilla_map_decoration_types, MapDecorationTypeCatalogEntry, VanillaMapDecorationType,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemFrameMapDecorationTexture {
    pub sprite_id: String,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}
