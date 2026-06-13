use bbb_protocol::codec::ProtocolError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WorldDecodeError {
    #[error(transparent)]
    Protocol(#[from] ProtocolError),
    #[error("invalid paletted container bits_per_entry {0}")]
    InvalidPalettedBits(u8),
    #[error("chunk section buffer has {actual} bytes, max is {max}")]
    ChunkSectionBufferTooLarge { actual: usize, max: usize },
    #[error("byte array has {actual} bytes, max is {max}")]
    ByteArrayTooLarge { actual: usize, max: usize },
    #[error("biome update has {remaining} trailing bytes")]
    TrailingBiomeData { remaining: usize },
    #[error("block entity data has {remaining} trailing bytes")]
    TrailingBlockEntityData { remaining: usize },
    #[error("negative NBT array length {0}")]
    NegativeNbtArrayLength(i32),
    #[error("invalid NBT tag id {0}")]
    InvalidNbtTag(u8),
}

pub type Result<T> = std::result::Result<T, WorldDecodeError>;
