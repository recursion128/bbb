use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, RwLock},
};

use bbb_world::WorldStore;
use serde::{Deserialize, Serialize};

use super::{AudioCounters, NetCounters, RendererCounters};

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
    SetHeldSlot {
        slot: u8,
    },
    SetFlying {
        flying: bool,
    },
    PlaceRecipe {
        container_id: i32,
        recipe_index: i32,
        use_max_items: bool,
    },
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
    ContainerClick(ContainerClickControlRequest),
    ContainerClose {
        container_id: i32,
    },
    ContainerSlotStateChanged {
        slot_id: i32,
        container_id: i32,
        new_state: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerClickControlRequest {
    pub container_id: i32,
    pub state_id: i32,
    pub slot_num: i16,
    pub button_num: i8,
    pub input: ContainerInputControl,
    #[serde(default)]
    pub changed_slots: Vec<ContainerChangedSlotControl>,
    #[serde(default)]
    pub carried_item: HashedStackControl,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerChangedSlotControl {
    pub slot: i16,
    pub stack: HashedStackControl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContainerInputControl {
    Pickup,
    QuickMove,
    Swap,
    Clone,
    Throw,
    QuickCraft,
    PickupAll,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum HashedStackControl {
    #[default]
    Empty,
    Item {
        item_id: i32,
        count: i32,
        #[serde(default)]
        components: HashedComponentPatchControl,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HashedComponentPatchControl {
    #[serde(default, deserialize_with = "deserialize_i32_key_map")]
    pub added_components: BTreeMap<i32, i32>,
    #[serde(default)]
    pub removed_components: BTreeSet<i32>,
}

fn deserialize_i32_key_map<'de, D>(deserializer: D) -> Result<BTreeMap<i32, i32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = BTreeMap::<String, i32>::deserialize(deserializer)?;
    raw.into_iter()
        .map(|(key, value)| {
            let key = key.parse::<i32>().map_err(|err| {
                serde::de::Error::custom(format!("invalid i32 map key {key:?}: {err}"))
            })?;
            Ok((key, value))
        })
        .collect()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ControlSnapshot {
    pub app: AppStatus,
    pub net: NetCounters,
    pub audio: AudioCounters,
    pub renderer: RendererCounters,
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
