//! Shared block/sky light storage and propagation core used by both client
//! local relighting and authoritative server lighting.

pub mod light_engine;
pub mod light_storage;

pub use light_storage::SavedLightSection;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LightError {
    #[error("light engine: {0}")]
    Engine(String),
    #[error("clientbound light [{chunk_x},{chunk_z}]: {detail}")]
    PacketLight {
        chunk_x: i32,
        chunk_z: i32,
        detail: String,
    },
}

pub type LightResult<T> = std::result::Result<T, LightError>;
