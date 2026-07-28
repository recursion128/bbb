use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    sync::{Arc, Mutex, OnceLock},
};

use arc_swap::ArcSwap;
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
    ChatMessage {
        message: String,
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

/// Synthetic local-player input driven over the control API. Requests are
/// replayed through the exact native handlers real window events use
/// (`handle_key_input` / `handle_mouse_motion`), so focus, screen and sprint
/// side effects stay on the canonical input path.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum InputControlRequest {
    Focus {
        focused: bool,
    },
    Key {
        key: InputKeyControl,
        pressed: bool,
    },
    LookDelta {
        dx: f64,
        dy: f64,
    },
    MouseButton {
        button: InputMouseButtonControl,
        pressed: bool,
    },
}

/// One ordered control-plane write submitted to the native simulation owner.
///
/// The control crate owns these external DTOs, while the installed sink owns
/// their runtime ordering and application. Keeping all write families in one
/// enum prevents the native runtime from polling independent per-family
/// vectors and accidentally reordering requests from concurrent producers.
#[derive(Debug, Clone, PartialEq)]
pub enum ControlWriteRequest {
    Shutdown,
    Screenshot { path: String },
    CodeOfConduct(CodeOfConductControlRequest),
    Net(NetControlRequest),
    Input(InputControlRequest),
}

/// Runtime-installed destination for ordered control writes.
pub trait ControlRequestSink: Send + Sync {
    /// Submits one request and returns the destination's pending depth.
    fn submit(&self, request: ControlWriteRequest) -> usize;
}

/// In-world mouse buttons the control API can drive; replayed through the
/// gameplay click handler only while no screen owns the cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputMouseButtonControl {
    Left,
    Right,
    Middle,
}

impl InputMouseButtonControl {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "left" => Some(Self::Left),
            "right" => Some(Self::Right),
            "middle" => Some(Self::Middle),
            _ => None,
        }
    }
}

/// Semantic movement keys the control API can drive; the native runtime maps
/// each to the default vanilla key binding's physical key code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputKeyControl {
    Forward,
    Backward,
    Left,
    Right,
    Jump,
    Sneak,
    Sprint,
    Hotbar1,
    Hotbar2,
    Hotbar3,
    Hotbar4,
    Hotbar5,
    Hotbar6,
    Hotbar7,
    Hotbar8,
    Hotbar9,
    Inventory,
}

impl InputKeyControl {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "forward" => Some(Self::Forward),
            "backward" => Some(Self::Backward),
            "left" => Some(Self::Left),
            "right" => Some(Self::Right),
            "jump" => Some(Self::Jump),
            "sneak" => Some(Self::Sneak),
            "sprint" => Some(Self::Sprint),
            "hotbar1" => Some(Self::Hotbar1),
            "hotbar2" => Some(Self::Hotbar2),
            "hotbar3" => Some(Self::Hotbar3),
            "hotbar4" => Some(Self::Hotbar4),
            "hotbar5" => Some(Self::Hotbar5),
            "hotbar6" => Some(Self::Hotbar6),
            "hotbar7" => Some(Self::Hotbar7),
            "hotbar8" => Some(Self::Hotbar8),
            "hotbar9" => Some(Self::Hotbar9),
            "inventory" => Some(Self::Inventory),
            _ => None,
        }
    }
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

/// Immutable control-plane view published atomically by the native runtime.
/// World-owned state is published separately through [`SharedWorld`] at the
/// same `generation`, keeping runtime counters out of canonical state.
#[derive(Debug, Clone, Default)]
pub struct ControlSnapshot {
    pub generation: u64,
    pub app: AppStatus,
    pub net: NetCounters,
    pub audio: AudioCounters,
    pub renderer: RendererCounters,
}

pub type SharedSnapshot = Arc<ArcSwap<ControlSnapshot>>;

/// Immutable world generation paired with [`ControlSnapshot::generation`].
/// Native publication reuses the exact immutable `Arc<WorldStore>` consumed by
/// the renderer. The live mutable canonical owner remains private to native's
/// simulation runtime and is never shared with control readers.
#[derive(Debug, Clone, Default)]
pub struct ControlWorldSnapshot {
    pub generation: u64,
    pub world: Arc<WorldStore>,
}

pub type SharedWorld = Arc<ArcSwap<ControlWorldSnapshot>>;

/// Control-plane input queues: produced by the control API server (and the
/// native code-of-conduct overlay) and drained by the native runtime each
/// frame. Kept separate from [`ControlSnapshot`] so the published snapshot
/// stays a pure derived view.
#[derive(Debug, Default)]
pub struct ControlRequests {
    pub shutdown_requested: bool,
    pub screenshot_request: Option<String>,
    pub code_of_conduct_requests: Vec<CodeOfConductControlRequest>,
    pub net_requests: Vec<NetControlRequest>,
    pub input_requests: Vec<InputControlRequest>,
}

/// Shared control write producer.
///
/// Unit tests and standalone control users retain inspectable compatibility
/// queues. The native client installs its simulation mailbox before starting
/// the control server, after which every write goes directly to that one
/// ordered destination.
#[derive(Clone, Default)]
pub struct SharedControlRequests {
    compatibility: Arc<Mutex<ControlRequests>>,
    sink: Arc<OnceLock<Arc<dyn ControlRequestSink>>>,
}

impl SharedControlRequests {
    pub fn lock(&self) -> std::sync::LockResult<std::sync::MutexGuard<'_, ControlRequests>> {
        self.compatibility.lock()
    }

    pub fn install_sink(&self, sink: Arc<dyn ControlRequestSink>) {
        let compatibility = self
            .compatibility
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        assert!(
            !compatibility.shutdown_requested
                && compatibility.screenshot_request.is_none()
                && compatibility.code_of_conduct_requests.is_empty()
                && compatibility.net_requests.is_empty()
                && compatibility.input_requests.is_empty(),
            "control request sink must be installed before producers start"
        );
        self.sink
            .set(sink)
            .unwrap_or_else(|_| panic!("control request sink may only be installed once"));
    }

    pub fn submit(&self, request: ControlWriteRequest) -> usize {
        if let Some(sink) = self.sink.get() {
            return sink.submit(request);
        }

        let mut compatibility = self
            .compatibility
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        // Installation holds the compatibility mutex while publishing the OnceLock.
        // Rechecking here closes the only transition race for a producer that
        // observed the empty OnceLock immediately before installation.
        if let Some(sink) = self.sink.get() {
            drop(compatibility);
            return sink.submit(request);
        }
        match request {
            ControlWriteRequest::Shutdown => {
                compatibility.shutdown_requested = true;
                usize::from(compatibility.shutdown_requested)
            }
            ControlWriteRequest::Screenshot { path } => {
                compatibility.screenshot_request = Some(path);
                usize::from(compatibility.screenshot_request.is_some())
            }
            ControlWriteRequest::CodeOfConduct(request) => {
                compatibility.code_of_conduct_requests.push(request);
                compatibility.code_of_conduct_requests.len()
            }
            ControlWriteRequest::Net(request) => {
                compatibility.net_requests.push(request);
                compatibility.net_requests.len()
            }
            ControlWriteRequest::Input(request) => {
                compatibility.input_requests.push(request);
                compatibility.input_requests.len()
            }
        }
    }
}

impl fmt::Debug for SharedControlRequests {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SharedControlRequests")
            .field("compatibility", &self.compatibility)
            .field("sink_installed", &self.sink.get().is_some())
            .finish()
    }
}

/// Bundle of the shared control-plane handles served by the control API.
#[derive(Debug, Clone, Default)]
pub struct ControlState {
    pub snapshot: SharedSnapshot,
    pub requests: SharedControlRequests,
    pub world: SharedWorld,
}

impl ControlState {
    pub fn new(version: impl Into<String>) -> Self {
        Self {
            snapshot: Arc::new(ArcSwap::from_pointee(ControlSnapshot {
                app: AppStatus {
                    version: version.into(),
                    running: true,
                },
                ..ControlSnapshot::default()
            })),
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn struct_fields<'a>(source: &'a str, declaration: &str) -> Vec<&'a str> {
        source
            .split_once(declaration)
            .expect("struct declaration is present")
            .1
            .split_once('}')
            .expect("struct body is closed")
            .0
            .lines()
            .filter_map(|line| {
                let field = line.trim().strip_prefix("pub ").unwrap_or(line.trim());
                let (name, _) = field.split_once(':')?;
                name.chars()
                    .all(|character| character == '_' || character.is_ascii_alphanumeric())
                    .then_some(name)
            })
            .collect()
    }

    #[derive(Default)]
    struct RecordingSink {
        requests: Mutex<Vec<ControlWriteRequest>>,
    }

    impl ControlRequestSink for RecordingSink {
        fn submit(&self, request: ControlWriteRequest) -> usize {
            let mut requests = self
                .requests
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            requests.push(request);
            requests.len()
        }
    }

    #[test]
    fn installed_sink_receives_writes_without_touching_compatibility_queues() {
        let requests = SharedControlRequests::default();
        let sink = Arc::new(RecordingSink::default());
        requests.install_sink(sink.clone());

        assert_eq!(
            requests.submit(ControlWriteRequest::Screenshot {
                path: "ordered.png".to_string(),
            }),
            1
        );
        assert_eq!(
            *sink
                .requests
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
            [ControlWriteRequest::Screenshot {
                path: "ordered.png".to_string(),
            }]
        );
        let compatibility = requests
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        assert!(compatibility.screenshot_request.is_none());
        assert!(compatibility.net_requests.is_empty());
        assert!(compatibility.input_requests.is_empty());
    }

    #[test]
    fn control_snapshot_shapes_keep_world_and_runtime_owners_separate() {
        let source = include_str!("snapshot.rs");
        assert_eq!(
            struct_fields(source, "pub struct ControlSnapshot {"),
            ["generation", "app", "net", "audio", "renderer"]
        );
        assert_eq!(
            struct_fields(source, "pub struct ControlWorldSnapshot {"),
            ["generation", "world"]
        );
    }
}
