#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemFrameMapDecorationTexture {
    pub sprite_id: String,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}
