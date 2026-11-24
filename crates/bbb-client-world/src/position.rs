pub use bbb_protocol::packets::{BlockPos, ChunkPos};

pub fn section_block_index(x: u8, y: u8, z: u8) -> usize {
    ((y as usize) << 8) | ((z as usize) << 4) | x as usize
}

pub fn section_biome_index(x: u8, y: u8, z: u8) -> usize {
    ((y as usize) << 4) | ((z as usize) << 2) | x as usize
}
