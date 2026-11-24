use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    sync::Arc,
};

use arc_swap::ArcSwap;
use bbb_client_world::WorldStore;
use serde::{Deserialize, Serialize};

use super::{AudioCounters, NetCounters, RendererCounters};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppStatus {
    pub version: String,
    pub running: bool,
    #[serde(default)]
    pub cursor_gui_position: Option<[f64; 2]>,
    #[serde(default)]
    pub gui_surface_width: u32,
    #[serde(default)]
    pub gui_surface_height: u32,
    #[serde(default)]
    pub cursor_captured: bool,
    #[serde(default)]
    pub input_focused: bool,
    #[serde(default)]
    pub mouse_button_events: u64,
    #[serde(default)]
    pub left_mouse_button_events: u64,
    #[serde(default)]
    pub left_mouse_button_press_events: u64,
    #[serde(default)]
    pub inventory_mouse_button_events: u64,
    #[serde(default)]
    pub inventory_mouse_button_consumed_events: u64,
    #[serde(default)]
    pub inventory_recipe_book_button_hit_events: u64,
    #[serde(default)]
    pub keyboard_events: u64,
    #[serde(default)]
    pub keyboard_press_events: u64,
    #[serde(default)]
    pub key_t_events: u64,
    #[serde(default)]
    pub inventory_keyboard_events: u64,
    #[serde(default)]
    pub inventory_key_t_press_events: u64,
    #[serde(default)]
    pub recipe_book_search_focused: bool,
    #[serde(default)]
    pub recipe_book_search_text_char_count: usize,
    #[serde(default)]
    pub recipe_book_language_requests: u64,
    #[serde(default)]
    pub recipe_book_language_target_available: bool,
    #[serde(default)]
    pub recipe_book_language_transitions: u64,
    #[serde(default)]
    pub disconnected_screen: Option<DisconnectedScreenStatus>,
    #[serde(default)]
    pub chat_screen: Option<ChatScreenStatus>,
    #[serde(default)]
    pub death_screen: Option<DeathScreenStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisconnectedScreenStatus {
    pub generation: u64,
    pub title_translation_key: String,
    pub reason_preview: String,
    pub reason_retained_bytes: usize,
    pub reason_truncated: bool,
    pub report_to_server_available: bool,
    pub report_directory_available: bool,
}

/// Bounded immutable view of the native-owned ChatScreen lifecycle. Recent
/// messages and draft contents remain private; control receives only the
/// currently displayed edit value and scalar process-lifetime diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatScreenStatus {
    pub generation: u64,
    pub screen: ChatScreenControlKind,
    pub method: ChatMethodControl,
    pub input_text: String,
    pub input_utf16_units: usize,
    pub draft: bool,
    pub history_position: usize,
    pub recent_count: usize,
    pub has_latest_draft: bool,
    pub save_chat_drafts: bool,
    pub can_send_messages: bool,
    pub can_send_commands: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatScreenControlKind {
    Chat,
    Restrictions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatMethodControl {
    Message,
    Command,
}

/// Bounded immutable view of the native-owned DeathScreen decision state.
/// Cause text and report-draft contents stay in their canonical world owners.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeathScreenStatus {
    pub generation: u64,
    pub screen: DeathScreenControlKind,
    pub focused: DeathScreenControlFocus,
    pub elapsed_ticks: u32,
    pub buttons_active: bool,
    pub hardcore: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeathScreenControlKind {
    Main,
    QuitConfirmation,
    LinkConfirmation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeathScreenControlFocus {
    Respawn,
    TitleScreen,
    QuitTitleScreen,
    QuitRespawn,
    LinkOpen,
    LinkCopy,
    LinkCancel,
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LookAtAnchorControl {
    #[default]
    Center,
    Eyes,
    Feet,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "target_kind")]
pub enum LookAtTargetControl {
    Position {
        x: f64,
        y: f64,
        z: f64,
    },
    Entity {
        entity_id: i32,
        anchor: LookAtAnchorControl,
    },
}

/// Synthetic local-player input driven over the control API. Requests are
/// replayed through the exact native handlers real window events use
/// (`handle_key_input` / `handle_mouse_motion`), so focus, screen and sprint
/// side effects stay on the canonical input path.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum InputControlRequest {
    Focus {
        focused: bool,
    },
    FocusCapture {
        focused: bool,
        capture_cursor: bool,
    },
    Key {
        key: InputKeyControl,
        pressed: bool,
    },
    KeyHold {
        key: InputKeyControl,
        hold_ticks: u32,
    },
    Text {
        text: String,
    },
    LookDelta {
        dx: f64,
        dy: f64,
    },
    Look {
        yaw: f32,
        pitch: f32,
    },
    LookAt {
        target: LookAtTargetControl,
    },
    MouseButton {
        button: InputMouseButtonControl,
        pressed: bool,
    },
    MouseButtonHold {
        button: InputMouseButtonControl,
        hold_ticks: u32,
    },
    MouseTap {
        button: InputMouseButtonControl,
    },
    Scroll {
        dx: f32,
        dy: f32,
    },
    /// Moves the cursor to window pixel coordinates, as a window event would.
    Cursor {
        x: f64,
        y: f64,
    },
    /// Moves the cursor to window pixel coordinates, then presses and
    /// releases `button` there, as window events would.
    Click {
        x: f64,
        y: f64,
        button: InputMouseButtonControl,
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
    /// Wakes the simulation after the lockstep clock advanced.
    LockstepWake,
    /// Completes after the given number of real-time client ticks elapse.
    WaitTicks {
        ticks: u32,
    },
    Screenshot {
        path: String,
    },
    Recording(super::RecordingControlRequest),
    CodeOfConduct(CodeOfConductControlRequest),
    Net(NetControlRequest),
    Input(InputControlRequest),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlSubmitReceipt {
    pub pending: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlSubmitError {
    Closed,
}

impl fmt::Display for ControlSubmitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Closed => formatter.write_str("client command queue is closed"),
        }
    }
}

impl std::error::Error for ControlSubmitError {}

/// Runtime-owned destination for ordered control writes.
pub trait ControlRequestSink: Send + Sync {
    /// Submits one request and returns its accepted queue observation. A
    /// terminal destination must fail explicitly.
    fn submit(
        &self,
        request: ControlWriteRequest,
    ) -> Result<ControlSubmitReceipt, ControlSubmitError>;

    fn submit_confirmed(
        &self,
        _request: ControlWriteRequest,
        _completion: crate::control::ControlCompletionSender,
    ) -> Result<ControlSubmitReceipt, ControlSubmitError> {
        Err(ControlSubmitError::Closed)
    }
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

impl InputControlRequest {
    pub const MAX_TEXT_UTF16_UNITS: usize = 4_096;
}

macro_rules! input_keys {
    ($($variant:ident => $name:literal),+ $(,)?) => {
        /// Semantic keys mapped by the native runtime to default Vanilla bindings.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
        pub enum InputKeyControl {
            $(#[serde(rename = $name)] $variant,)+
        }

        impl InputKeyControl {
            pub const NAMES: &'static [&'static str] = &[$($name,)+];

            pub fn from_name(name: &str) -> Option<Self> {
                match name {
                    $($name => Some(Self::$variant),)+
                    _ => None,
                }
            }
        }
    };
}

input_keys! {
    Forward => "forward",
    Backward => "backward",
    Left => "left",
    Right => "right",
    Jump => "jump",
    Sneak => "sneak",
    Sprint => "sprint",
    Hotbar1 => "hotbar1",
    Hotbar2 => "hotbar2",
    Hotbar3 => "hotbar3",
    Hotbar4 => "hotbar4",
    Hotbar5 => "hotbar5",
    Hotbar6 => "hotbar6",
    Hotbar7 => "hotbar7",
    Hotbar8 => "hotbar8",
    Hotbar9 => "hotbar9",
    Inventory => "inventory",
    Chat => "chat",
    Command => "command",
    Enter => "enter",
    Escape => "escape",
    Up => "up",
    Down => "down",
    PageUp => "page_up",
    PageDown => "page_down",
    Backspace => "backspace",
    F1 => "f1",
    F3 => "f3",
    F4 => "f4",
    F5 => "f5",
    F6 => "f6",
    F9 => "f9",
    Tab => "tab",
    Drop => "drop",
    SwapOffhand => "swap_offhand",
    Advancements => "advancements",
    Social => "social",
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

/// Immutable control-plane view of one simulation generation.
#[derive(Debug, Clone, Default)]
pub struct ControlSnapshot {
    pub generation: u64,
    pub app: AppStatus,
    pub net: NetCounters,
    pub audio: AudioCounters,
    pub renderer: RendererCounters,
    pub(crate) world_view: ControlWorldSnapshot,
}

pub type SharedSnapshot = Arc<ArcSwap<ControlSnapshot>>;

/// Frozen world state and the UI projected by the same simulation step.
#[derive(Debug, Clone)]
pub(crate) struct ControlWorldSnapshot {
    pub client_ui: serde_json::Value,
    pub world: Arc<WorldStore>,
    pub creative_inventory: Option<serde_json::Value>,
}

impl Default for ControlWorldSnapshot {
    fn default() -> Self {
        let world = WorldStore::new();
        let client_ui = crate::input::ClientInputState::default()
            .dialog_control_projection(serde_json::to_value(world.client_ui()).unwrap());
        Self {
            world: Arc::new(world),
            creative_inventory: None,
            client_ui,
        }
    }
}

/// Shared control write producer.
///
/// Construction requires the runtime-owned ordered destination. There is no
/// pre-install compatibility queue or mutable control-owned command state.
#[derive(Clone)]
pub struct SharedControlRequests {
    sink: Arc<dyn ControlRequestSink>,
}

impl SharedControlRequests {
    pub fn new(sink: Arc<dyn ControlRequestSink>) -> Self {
        Self { sink }
    }

    pub fn submit(
        &self,
        request: ControlWriteRequest,
    ) -> Result<ControlSubmitReceipt, ControlSubmitError> {
        self.sink.submit(request)
    }

    pub fn submit_confirmed(
        &self,
        request: ControlWriteRequest,
        completion: crate::control::ControlCompletionSender,
    ) -> Result<ControlSubmitReceipt, ControlSubmitError> {
        self.sink.submit_confirmed(request, completion)
    }
}

impl fmt::Debug for SharedControlRequests {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SharedControlRequests")
            .field("sink", &"installed")
            .finish()
    }
}

const DEMAND_LEASE_MS: u64 = 5_000;

/// Coordinates lazy and rate-limited simulation snapshot publication between the
/// simulation event loop and control/MCP consumers.
#[derive(Debug, Default)]
pub struct ControlPublicationDemand {
    sim_attached: std::sync::atomic::AtomicBool,
    active_consumers: std::sync::atomic::AtomicUsize,
    dirty: std::sync::atomic::AtomicBool,
    flush_requested: std::sync::atomic::AtomicBool,
    demand_lease_until_ms: std::sync::atomic::AtomicU64,
    origin: std::sync::OnceLock<bbb_client_clock::Instant>,
}

/// Keeps control snapshot demand active while a streaming control session is open.
#[derive(Debug)]
pub struct ControlConsumerGuard {
    demand: Arc<ControlPublicationDemand>,
}

impl Drop for ControlConsumerGuard {
    fn drop(&mut self) {
        self.demand
            .active_consumers
            .fetch_sub(1, std::sync::atomic::Ordering::AcqRel);
    }
}

impl ControlPublicationDemand {
    fn elapsed_ms_at(&self, now: bbb_client_clock::Instant) -> u64 {
        let origin = self.origin.get_or_init(|| now);
        u64::try_from(now.saturating_duration_since(*origin).as_millis())
            .unwrap_or(u64::MAX - DEMAND_LEASE_MS)
            .saturating_add(1)
    }

    pub fn attach_simulation(&self) {
        self.sim_attached
            .store(true, std::sync::atomic::Ordering::Release);
    }

    pub fn detach_simulation(&self) {
        self.sim_attached
            .store(false, std::sync::atomic::Ordering::Release);
    }

    pub fn enter_consumer(self: &Arc<Self>) -> ControlConsumerGuard {
        self.active_consumers
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        ControlConsumerGuard {
            demand: Arc::clone(self),
        }
    }

    pub fn record_demand(&self) {
        let until = self
            .elapsed_ms_at(bbb_client_clock::Instant::now())
            .saturating_add(DEMAND_LEASE_MS);
        self.demand_lease_until_ms
            .fetch_max(until, std::sync::atomic::Ordering::AcqRel);
    }

    pub fn has_demand_at(&self, now: bbb_client_clock::Instant) -> bool {
        if self
            .active_consumers
            .load(std::sync::atomic::Ordering::Acquire)
            > 0
        {
            return true;
        }
        let now_ms = self.elapsed_ms_at(now);
        now_ms
            <= self
                .demand_lease_until_ms
                .load(std::sync::atomic::Ordering::Acquire)
    }

    pub fn mark_dirty(&self) {
        self.dirty.store(true, std::sync::atomic::Ordering::Release);
    }

    pub fn clear_dirty(&self) {
        self.dirty
            .store(false, std::sync::atomic::Ordering::Release);
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty.load(std::sync::atomic::Ordering::Acquire)
    }

    pub fn request_flush(&self) {
        self.flush_requested
            .store(true, std::sync::atomic::Ordering::Release);
    }

    pub fn take_flush_requested(&self) -> bool {
        self.flush_requested
            .swap(false, std::sync::atomic::Ordering::AcqRel)
    }

    pub fn needs_flush(&self) -> bool {
        self.sim_attached.load(std::sync::atomic::Ordering::Acquire)
            && self.dirty.load(std::sync::atomic::Ordering::Acquire)
    }
}

/// Bundle of the shared control-plane handles served by the control API.
#[derive(Debug, Clone)]
pub struct ControlState {
    pub snapshot: SharedSnapshot,
    pub requests: SharedControlRequests,
    pub publication: Arc<ControlPublicationDemand>,
}

impl ControlState {
    pub fn new(version: impl Into<String>, requests: SharedControlRequests) -> Self {
        Self {
            snapshot: Arc::new(ArcSwap::from_pointee(ControlSnapshot {
                app: AppStatus {
                    version: version.into(),
                    running: true,
                    disconnected_screen: None,
                    chat_screen: None,
                    death_screen: None,
                    ..AppStatus::default()
                },
                ..ControlSnapshot::default()
            })),
            requests,
            publication: Arc::new(ControlPublicationDemand::default()),
        }
    }

    pub async fn ensure_fresh_snapshot(&self) {
        self.publication.record_demand();
        if !self.publication.needs_flush() {
            return;
        }
        self.publication.request_flush();
        let (sender, answer) = tokio::sync::oneshot::channel();
        if self
            .requests
            .submit_confirmed(ControlWriteRequest::LockstepWake, sender)
            .is_ok()
        {
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = tokio::time::timeout(std::time::Duration::from_millis(100), answer).await;
            }
            #[cfg(target_arch = "wasm32")]
            {
                let _ = answer.await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct RecordingSink {
        requests: std::sync::Mutex<Vec<ControlWriteRequest>>,
    }

    impl ControlRequestSink for RecordingSink {
        fn submit(
            &self,
            request: ControlWriteRequest,
        ) -> Result<ControlSubmitReceipt, ControlSubmitError> {
            let mut requests = self
                .requests
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            requests.push(request);
            Ok(ControlSubmitReceipt {
                pending: requests.len(),
            })
        }
    }

    #[test]
    fn constructed_sink_receives_every_write_without_transition_state() {
        let sink = Arc::new(RecordingSink::default());
        let requests = SharedControlRequests::new(sink.clone());
        let submitted = [
            ControlWriteRequest::Shutdown,
            ControlWriteRequest::Screenshot {
                path: "ordered.png".to_string(),
            },
            ControlWriteRequest::CodeOfConduct(CodeOfConductControlRequest::ClearAcceptance),
            ControlWriteRequest::Net(NetControlRequest::PerformRespawn),
            ControlWriteRequest::Input(InputControlRequest::Focus { focused: true }),
        ];

        for (index, request) in submitted.iter().cloned().enumerate() {
            assert_eq!(requests.submit(request).unwrap().pending, index + 1);
        }
        assert_eq!(
            *sink
                .requests
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
            submitted
        );
    }
}
