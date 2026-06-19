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
    ChangeDifficulty {
        difficulty: DifficultyControl,
    },
    ChangeGameMode {
        game_mode: GameModeControl,
    },
    LockDifficulty {
        locked: bool,
    },
    SetHeldSlot {
        slot: u8,
    },
    SetFlying {
        flying: bool,
    },
    PerformRespawn,
    RequestStats,
    RequestGameRuleValues,
    PlaceRecipe {
        container_id: i32,
        recipe_index: i32,
        use_max_items: bool,
    },
    ChangeRecipeBookSettings {
        book_type: RecipeBookTypeControl,
        open: bool,
        filtering: bool,
    },
    MarkRecipeSeen {
        recipe_index: i32,
    },
    RenameItem {
        name: String,
    },
    EditBook {
        slot: i32,
        pages: Vec<String>,
        title: Option<String>,
    },
    SignUpdate {
        x: i32,
        y: i32,
        z: i32,
        is_front_text: bool,
        lines: [String; 4],
    },
    OpenAdvancementsTab {
        tab: String,
    },
    CloseAdvancementsScreen,
    SelectTrade {
        item: i32,
    },
    SetBeacon {
        primary_effect: Option<i32>,
        secondary_effect: Option<i32>,
    },
    SetCreativeModeSlot(CreativeModeSlotControlRequest),
    SelectBundleItem {
        slot_id: i32,
        selected_item_index: i32,
    },
    ChatCommand {
        command: String,
    },
    CommandSuggestionRequest {
        id: i32,
        command: String,
    },
    QueryBlockEntityTag {
        transaction_id: i32,
        x: i32,
        y: i32,
        z: i32,
    },
    QueryEntityTag {
        transaction_id: i32,
        entity_id: i32,
    },
    SpectateEntity {
        entity_id: i32,
    },
    TeleportToEntity {
        uuid: uuid::Uuid,
    },
    ContainerButtonClick {
        container_id: i32,
        button_id: i32,
    },
    ContainerClickSlot(ContainerClickSlotControlRequest),
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

impl NetControlRequest {
    pub fn change_difficulty_named(difficulty: &str) -> Option<Self> {
        let difficulty = match difficulty {
            "peaceful" => DifficultyControl::Peaceful,
            "easy" => DifficultyControl::Easy,
            "normal" => DifficultyControl::Normal,
            "hard" => DifficultyControl::Hard,
            _ => return None,
        };
        Some(Self::ChangeDifficulty { difficulty })
    }

    pub fn change_game_mode_named(game_mode: &str) -> Option<Self> {
        let game_mode = match game_mode {
            "survival" => GameModeControl::Survival,
            "creative" => GameModeControl::Creative,
            "adventure" => GameModeControl::Adventure,
            "spectator" => GameModeControl::Spectator,
            _ => return None,
        };
        Some(Self::ChangeGameMode { game_mode })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DifficultyControl {
    Peaceful,
    Easy,
    Normal,
    Hard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameModeControl {
    Survival,
    Creative,
    Adventure,
    Spectator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecipeBookTypeControl {
    Crafting,
    Furnace,
    BlastFurnace,
    Smoker,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreativeModeSlotControlRequest {
    pub slot_num: i16,
    #[serde(default)]
    pub item: CreativeModeItemStackControl,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum CreativeModeItemStackControl {
    #[default]
    Empty,
    Item {
        item_id: i32,
        count: i32,
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
pub struct ContainerClickSlotControlRequest {
    pub slot_num: i16,
    pub button_num: i8,
    pub input: ContainerInputControl,
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
