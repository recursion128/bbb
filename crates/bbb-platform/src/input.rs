use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InputSnapshot {
    pub close_requested: bool,
    pub focused: bool,
}
