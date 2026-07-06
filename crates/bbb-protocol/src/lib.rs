mod component;

pub use component::{
    decode_sign_block_entity_nbt, decode_styled_component_summary, styled_runs_summary_text,
    ComponentStyle, SignBlockEntityNbt, SignTextNbt, StyledTextRun,
};

pub mod codec;
pub mod entity_types;
pub mod frame;
pub mod ids;
pub mod packets;

pub const MC_VERSION: &str = "26.1";
pub const PROTOCOL_VERSION: i32 = 775;
