use std::sync::{Arc, RwLock};

use bbb_world::{WorldCounters, WorldStore};
use serde::{Deserialize, Serialize};

use super::{NetCounters, RendererCounters};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppStatus {
    pub version: String,
    pub running: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CodeOfConductControlRequest {
    Accept { remember: bool },
    Decline,
    ClearAcceptance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetControlRequest {
    ChatCommand {
        command: String,
    },
    CommandSuggestionRequest {
        id: i32,
        command: String,
    },
    ContainerButtonClick {
        container_id: i32,
        button_id: i32,
    },
    ContainerSlotStateChanged {
        slot_id: i32,
        container_id: i32,
        new_state: bool,
    },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ControlSnapshot {
    pub app: AppStatus,
    pub net: NetCounters,
    pub renderer: RendererCounters,
    pub world: WorldCounters,
    #[serde(skip)]
    pub screenshot_request: Option<String>,
    #[serde(skip)]
    pub code_of_conduct_requests: Vec<CodeOfConductControlRequest>,
    #[serde(skip)]
    pub net_requests: Vec<NetControlRequest>,
    #[serde(skip)]
    pub world_store: WorldStore,
}

pub type SharedSnapshot = Arc<RwLock<ControlSnapshot>>;
