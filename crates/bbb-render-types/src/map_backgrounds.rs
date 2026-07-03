#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FirstPersonMapBackgroundKind {
    Plain,
    Checkerboard,
}

impl FirstPersonMapBackgroundKind {
    pub fn vanilla_path(self) -> &'static str {
        match self {
            Self::Plain => "minecraft:textures/map/map_background.png",
            Self::Checkerboard => "minecraft:textures/map/map_background_checkerboard.png",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirstPersonMapBackgroundTexture {
    pub kind: FirstPersonMapBackgroundKind,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}
