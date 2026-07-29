use bbb_protocol::{
    packets::{
        CraftingRecipeDisplaySummary, DialogHolder, ItemStackSummary as ProtocolItemStackSummary,
        MountScreenOpen as ProtocolMountScreenOpen, OpenSignEditor as ProtocolOpenSignEditor,
        PlaceGhostRecipe as ProtocolPlaceGhostRecipe, PongResponse as ProtocolPongResponse,
        RecipeDisplaySummary as ProtocolRecipeDisplaySummary, ShowDialog as ProtocolShowDialog,
    },
    NbtSummaryValue,
};
use serde::{Deserialize, Serialize};

use crate::{
    client::book::{BookScreenState, OpenBookState},
    client::dialog::{
        decode_component_click_dialog, decode_dialog_registry_entry, CommonDialogDataState,
        DialogBodyState, DialogDataState, DialogInputControlState, DialogState, DialogTextState,
    },
    client::inventory::{RecipeUpdateListenerContext, RecipeUpdateListenerLayout},
    entities::{is_vanilla_abstract_horse_type, is_vanilla_abstract_nautilus_type},
    for_each_shaped_recipe_grid_slot, protocol_block_pos,
    recipes::slot_display_holders_are_resolved,
    BlockPos, WorldStore,
};

const DIALOG_WAITING_CLOSE_ACTIVE_TICKS: u32 = 101;
const MAX_GHOST_RECIPE_CANDIDATES: usize = 65_536;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientUiState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_creative_inventory: Option<crate::CreativeInventoryScreenState>,
    #[serde(default)]
    pub low_disk_space_warning_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_command_block_editor: Option<crate::CommandBlockEditorState>,
    #[serde(default)]
    pub command_block_editor_open_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_minecart_command_block_editor: Option<crate::MinecartCommandBlockEditorState>,
    #[serde(default)]
    pub minecart_command_block_editor_open_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_structure_block_editor: Option<crate::StructureBlockEditorState>,
    #[serde(default)]
    pub structure_block_editor_open_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_jigsaw_block_editor: Option<crate::JigsawBlockEditorState>,
    #[serde(default)]
    pub jigsaw_block_editor_open_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_test_block_editor: Option<crate::TestBlockEditorState>,
    #[serde(default)]
    pub test_block_editor_open_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_test_instance_block_editor: Option<crate::TestInstanceBlockEditorState>,
    #[serde(default)]
    pub test_instance_block_editor_open_count: usize,
    #[serde(default)]
    pub current_dialog: Option<DialogState>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dialog_input_values: Vec<Option<DialogInputValueState>>,
    #[serde(skip)]
    dialog_dragging_number_range_input: Option<usize>,
    #[serde(skip)]
    dialog_focus: Option<DialogFocusState>,
    #[serde(skip)]
    dialog_number_range_keyboard_input: Option<usize>,
    #[serde(skip)]
    dialog_text_edit: Option<DialogTextEditState>,
    #[serde(skip)]
    dialog_dragging_text_input: Option<usize>,
    #[serde(skip)]
    dialog_dragging_multiline_scroll_input: Option<usize>,
    #[serde(skip)]
    dialog_dragging_multiline_scroll_y: Option<i32>,
    #[serde(skip)]
    dialog_body_scroll_amount_bits: u64,
    #[serde(skip)]
    dialog_body_scroll_drag_y: Option<i32>,
    #[serde(skip)]
    dialog_warning_open: bool,
    #[serde(skip)]
    dialog_warning_focused_button: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dialog_link_confirmation: Option<DialogLinkConfirmationState>,
    #[serde(skip)]
    dialog_link_focused_button: Option<usize>,
    #[serde(skip)]
    dialog_link_return_stack: Vec<DialogLinkReturnState>,
    #[serde(skip)]
    dialog_open_url_requests: Vec<String>,
    #[serde(skip)]
    dialog_chat_links_disabled: bool,
    #[serde(skip)]
    dialog_chat_links_prompt_disabled: bool,
    #[serde(skip)]
    dialog_screen_revision: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dialog_previous_screen: Option<DialogPreviousScreenState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dialog_waiting_ticks: Option<u32>,
    #[serde(default)]
    pub current_book: Option<BookScreenState>,
    #[serde(default)]
    pub book_screen_generation: u64,
    #[serde(default)]
    pub current_advancements_screen: bool,
    #[serde(default)]
    pub current_stats_screen: bool,
    #[serde(default)]
    pub stats_screen_loading: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_social_interactions_screen: Option<crate::SocialInteractionsScreenState>,
    #[serde(default)]
    pub social_interactions_screen_generation: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_player_reporting_screen: Option<crate::PlayerReportingScreenState>,
    #[serde(default)]
    pub player_reporting_screen_generation: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_reporting_previous_social_screen: Option<crate::SocialInteractionsScreenState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_game_rules_screen: Option<crate::GameRulesScreenState>,
    #[serde(default)]
    pub game_rules_screen_generation: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_game_event_screen: Option<ClientGameEventScreenState>,
    #[serde(default)]
    pub game_event_screen_generation: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_death_screen: Option<crate::ClientDeathScreenState>,
    #[serde(default)]
    pub death_screen_generation: u64,
    #[serde(default)]
    pub last_code_of_conduct: Option<CodeOfConductState>,
    #[serde(default)]
    pub last_mount_screen: Option<MountScreenState>,
    #[serde(default)]
    pub last_open_book: Option<OpenBookState>,
    #[serde(default)]
    pub last_open_sign_editor: Option<OpenSignEditorState>,
    #[serde(default)]
    pub last_ghost_recipe: Option<GhostRecipeState>,
    #[serde(default)]
    pub last_pong_response: Option<PongResponseState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DialogPreviousScreenState {
    Book {
        screen: BookScreenState,
    },
    Advancements,
    Stats,
    SocialInteractions {
        screen: crate::SocialInteractionsScreenState,
    },
    PlayerReporting {
        screen: crate::PlayerReportingScreenState,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ClientGameEventScreenState {
    Win { generation: u64 },
    DemoHelp { generation: u64 },
}

impl ClientGameEventScreenState {
    pub fn generation(self) -> u64 {
        match self {
            Self::Win { generation } | Self::DemoHelp { generation } => generation,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogLinkConfirmationState {
    pub url: String,
    pub after_action: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DialogLinkReturnState {
    confirmation: DialogLinkConfirmationState,
    focused_button: Option<usize>,
    dialog: Option<DialogState>,
    input_values: Vec<Option<DialogInputValueState>>,
    dragging_number_range_input: Option<usize>,
    focus: Option<DialogFocusState>,
    number_range_keyboard_input: Option<usize>,
    text_edit: Option<DialogTextEditState>,
    dragging_text_input: Option<usize>,
    dragging_multiline_scroll_input: Option<usize>,
    dragging_multiline_scroll_y: Option<i32>,
    body_scroll_amount_bits: u64,
    body_scroll_drag_y: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DialogInputValueState {
    Boolean { selected: bool },
    NumberRange { slider_value_bits: u64 },
    SingleOption { selected_index: usize },
    Text { value: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DialogTextEditState {
    pub input_index: usize,
    pub cursor: usize,
    pub selection: usize,
    pub scroll_amount: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogBodyFocusPartState {
    Primary,
    Description,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogFocusState {
    WarningButton,
    Body {
        index: usize,
        part: DialogBodyFocusPartState,
    },
    Input {
        index: usize,
    },
    Action {
        index: usize,
    },
}

impl ClientUiState {
    fn advance_dialog_screen_revision(&mut self) {
        self.dialog_screen_revision = self.dialog_screen_revision.saturating_add(1);
    }

    fn show_dialog(&mut self, dialog: DialogState) {
        self.current_game_rules_screen = None;
        if self.dialog_link_confirmation.is_some() {
            self.suspend_dialog_link_confirmation();
            self.show_dialog_above_link_confirmation(dialog);
            return;
        }
        if !self.dialog_link_return_stack.is_empty() {
            self.show_dialog_above_link_confirmation(dialog);
            return;
        }
        if self.current_dialog.is_none() && self.dialog_waiting_ticks.is_none() {
            self.dialog_previous_screen = if let Some(screen) = self.current_book.take() {
                Some(DialogPreviousScreenState::Book { screen })
            } else if std::mem::take(&mut self.current_advancements_screen) {
                Some(DialogPreviousScreenState::Advancements)
            } else if std::mem::take(&mut self.current_stats_screen) {
                Some(DialogPreviousScreenState::Stats)
            } else if let Some(screen) = self.current_social_interactions_screen.take() {
                Some(DialogPreviousScreenState::SocialInteractions { screen })
            } else if let Some(screen) = self.current_player_reporting_screen.take() {
                Some(DialogPreviousScreenState::PlayerReporting { screen })
            } else {
                None
            };
        }
        self.dialog_waiting_ticks = None;
        self.dialog_dragging_number_range_input = None;
        self.dialog_focus = None;
        self.dialog_number_range_keyboard_input = None;
        self.dialog_text_edit = None;
        self.dialog_dragging_text_input = None;
        self.dialog_dragging_multiline_scroll_input = None;
        self.dialog_dragging_multiline_scroll_y = None;
        self.dialog_body_scroll_amount_bits = 0.0f64.to_bits();
        self.dialog_body_scroll_drag_y = None;
        self.dialog_link_focused_button = None;
        self.dialog_input_values = initial_dialog_input_values(&dialog);
        self.current_dialog = Some(dialog);
        self.advance_dialog_screen_revision();
    }

    fn clear_dialog(&mut self) -> bool {
        if self.dialog_link_confirmation.is_some() {
            return false;
        }
        if self.current_dialog.take().is_none() {
            return false;
        }
        self.dialog_dragging_number_range_input = None;
        self.dialog_focus = None;
        self.dialog_number_range_keyboard_input = None;
        self.dialog_text_edit = None;
        self.dialog_dragging_text_input = None;
        self.dialog_dragging_multiline_scroll_input = None;
        self.dialog_dragging_multiline_scroll_y = None;
        self.dialog_body_scroll_amount_bits = 0.0f64.to_bits();
        self.dialog_body_scroll_drag_y = None;
        self.dialog_link_focused_button = None;
        self.dialog_input_values.clear();
        if self.restore_suspended_dialog_link_confirmation() {
            self.advance_dialog_screen_revision();
            return true;
        }
        self.restore_dialog_previous_screen();
        self.advance_dialog_screen_revision();
        true
    }

    fn wait_for_dialog_response(&mut self) -> bool {
        if self.dialog_link_confirmation.is_some() {
            return false;
        }
        if self.current_dialog.take().is_none() {
            return false;
        }
        self.dialog_dragging_number_range_input = None;
        self.dialog_focus = None;
        self.dialog_number_range_keyboard_input = None;
        self.dialog_text_edit = None;
        self.dialog_dragging_text_input = None;
        self.dialog_dragging_multiline_scroll_input = None;
        self.dialog_dragging_multiline_scroll_y = None;
        self.dialog_body_scroll_amount_bits = 0.0f64.to_bits();
        self.dialog_body_scroll_drag_y = None;
        self.dialog_warning_open = false;
        self.dialog_warning_focused_button = None;
        self.dialog_link_focused_button = None;
        self.dialog_input_values.clear();
        self.dialog_waiting_ticks = Some(0);
        self.advance_dialog_screen_revision();
        true
    }

    fn close_dialog_waiting_response(&mut self) -> bool {
        if !self
            .dialog_waiting_ticks
            .is_some_and(|ticks| ticks >= DIALOG_WAITING_CLOSE_ACTIVE_TICKS)
        {
            return false;
        }
        self.dialog_waiting_ticks = None;
        if self.restore_suspended_dialog_link_confirmation() {
            self.advance_dialog_screen_revision();
            return true;
        }
        self.restore_dialog_previous_screen();
        self.advance_dialog_screen_revision();
        true
    }

    fn restore_dialog_previous_screen(&mut self) {
        match self.dialog_previous_screen.take() {
            Some(DialogPreviousScreenState::Book { screen }) => {
                self.current_book = Some(screen);
            }
            Some(DialogPreviousScreenState::Advancements) => {
                self.current_advancements_screen = true;
            }
            Some(DialogPreviousScreenState::Stats) => {
                self.current_stats_screen = true;
            }
            Some(DialogPreviousScreenState::SocialInteractions { screen }) => {
                self.current_social_interactions_screen = Some(screen);
            }
            Some(DialogPreviousScreenState::PlayerReporting { screen }) => {
                self.current_player_reporting_screen = Some(screen);
            }
            None => {}
        }
    }

    fn suspend_dialog_link_confirmation(&mut self) {
        let Some(confirmation) = self.dialog_link_confirmation.take() else {
            return;
        };
        self.dialog_link_return_stack.push(DialogLinkReturnState {
            confirmation,
            focused_button: self.dialog_link_focused_button.take(),
            dialog: self.current_dialog.take(),
            input_values: std::mem::take(&mut self.dialog_input_values),
            dragging_number_range_input: self.dialog_dragging_number_range_input.take(),
            focus: self.dialog_focus.take(),
            number_range_keyboard_input: self.dialog_number_range_keyboard_input.take(),
            text_edit: self.dialog_text_edit.take(),
            dragging_text_input: self.dialog_dragging_text_input.take(),
            dragging_multiline_scroll_input: self.dialog_dragging_multiline_scroll_input.take(),
            dragging_multiline_scroll_y: self.dialog_dragging_multiline_scroll_y.take(),
            body_scroll_amount_bits: std::mem::replace(
                &mut self.dialog_body_scroll_amount_bits,
                0.0f64.to_bits(),
            ),
            body_scroll_drag_y: self.dialog_body_scroll_drag_y.take(),
        });
    }

    fn show_dialog_above_link_confirmation(&mut self, dialog: DialogState) {
        self.dialog_waiting_ticks = None;
        self.dialog_dragging_number_range_input = None;
        self.dialog_focus = None;
        self.dialog_number_range_keyboard_input = None;
        self.dialog_text_edit = None;
        self.dialog_dragging_text_input = None;
        self.dialog_dragging_multiline_scroll_input = None;
        self.dialog_dragging_multiline_scroll_y = None;
        self.dialog_body_scroll_amount_bits = 0.0f64.to_bits();
        self.dialog_body_scroll_drag_y = None;
        self.dialog_link_focused_button = None;
        self.dialog_input_values = initial_dialog_input_values(&dialog);
        self.current_dialog = Some(dialog);
        self.advance_dialog_screen_revision();
    }

    fn restore_suspended_dialog_link_confirmation(&mut self) -> bool {
        let Some(state) = self.dialog_link_return_stack.pop() else {
            return false;
        };
        self.dialog_link_confirmation = Some(state.confirmation);
        self.dialog_link_focused_button = state.focused_button;
        self.current_dialog = state.dialog;
        self.dialog_input_values = state.input_values;
        self.dialog_dragging_number_range_input = state.dragging_number_range_input;
        self.dialog_focus = state.focus;
        self.dialog_number_range_keyboard_input = state.number_range_keyboard_input;
        self.dialog_text_edit = state.text_edit;
        self.dialog_dragging_text_input = state.dragging_text_input;
        self.dialog_dragging_multiline_scroll_input = state.dragging_multiline_scroll_input;
        self.dialog_dragging_multiline_scroll_y = state.dragging_multiline_scroll_y;
        self.dialog_body_scroll_amount_bits = state.body_scroll_amount_bits;
        self.dialog_body_scroll_drag_y = state.body_scroll_drag_y;
        true
    }

    pub(crate) fn replace_dialog(&mut self) {
        self.current_dialog = None;
        self.dialog_dragging_number_range_input = None;
        self.dialog_focus = None;
        self.dialog_number_range_keyboard_input = None;
        self.dialog_text_edit = None;
        self.dialog_dragging_text_input = None;
        self.dialog_dragging_multiline_scroll_input = None;
        self.dialog_dragging_multiline_scroll_y = None;
        self.dialog_body_scroll_amount_bits = 0.0f64.to_bits();
        self.dialog_body_scroll_drag_y = None;
        self.dialog_warning_open = false;
        self.dialog_warning_focused_button = None;
        self.dialog_link_confirmation = None;
        self.dialog_link_focused_button = None;
        self.dialog_link_return_stack.clear();
        self.dialog_input_values.clear();
        self.dialog_waiting_ticks = None;
        self.dialog_previous_screen = None;
        self.advance_dialog_screen_revision();
    }

    pub(crate) fn replace_screen(&mut self) {
        self.current_creative_inventory = None;
        self.replace_dialog();
        self.last_ghost_recipe = None;
        self.current_book = None;
        self.current_advancements_screen = false;
        self.current_stats_screen = false;
        self.current_social_interactions_screen = None;
        self.current_player_reporting_screen = None;
        self.player_reporting_previous_social_screen = None;
        self.current_game_rules_screen = None;
        self.current_game_event_screen = None;
        self.current_death_screen = None;
        self.current_command_block_editor = None;
        self.current_minecart_command_block_editor = None;
        self.current_structure_block_editor = None;
        self.current_jigsaw_block_editor = None;
        self.current_test_block_editor = None;
        self.current_test_instance_block_editor = None;
    }
}

fn initial_dialog_input_values(dialog: &DialogState) -> Vec<Option<DialogInputValueState>> {
    let Some(data) = dialog.data.as_ref() else {
        return Vec::new();
    };
    let common = common_dialog_data(data);
    common
        .input_data
        .iter()
        .map(|input| match &input.control {
            DialogInputControlState::Boolean { initial, .. } => {
                Some(DialogInputValueState::Boolean { selected: *initial })
            }
            DialogInputControlState::NumberRange { range, .. } => {
                let start = range.start();
                let end = range.end();
                let initial = range.initial().unwrap_or((start + end) / 2.0);
                let slider_value = if start == end {
                    0.5
                } else {
                    (initial - start) / (end - start)
                };
                Some(DialogInputValueState::NumberRange {
                    slider_value_bits: f64::from(slider_value).to_bits(),
                })
            }
            DialogInputControlState::SingleOption { options, .. } => {
                Some(DialogInputValueState::SingleOption {
                    selected_index: options
                        .iter()
                        .position(|option| option.initial)
                        .unwrap_or(0),
                })
            }
            DialogInputControlState::Text { initial, .. } => Some(DialogInputValueState::Text {
                value: initial.clone(),
            }),
        })
        .collect()
}

fn common_dialog_data(data: &DialogDataState) -> &CommonDialogDataState {
    match data {
        DialogDataState::Notice { common, .. }
        | DialogDataState::Confirmation { common, .. }
        | DialogDataState::MultiAction { common, .. }
        | DialogDataState::ServerLinks { common, .. }
        | DialogDataState::DialogList { common, .. } => common,
    }
}

const DIALOG_REGISTRY: &str = "minecraft:dialog";
const PAUSE_SCREEN_ADDITIONS_TAG: &str = "minecraft:pause_screen_additions";
const CUSTOM_OPTIONS_DIALOG: &str = "minecraft:custom_options";
const SERVER_LINKS_DIALOG: &str = "minecraft:server_links";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PauseScreenCustomDialogState {
    resource_id: String,
    external_title: DialogTextState,
    dialog: DialogState,
}

impl PauseScreenCustomDialogState {
    pub fn resource_id(&self) -> &str {
        &self.resource_id
    }

    pub fn external_title(&self) -> &DialogTextState {
        &self.external_title
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeOfConductState {
    pub text: String,
    pub text_hash: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MountScreenState {
    pub container_id: i32,
    pub inventory_columns: i32,
    pub entity_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenSignEditorState {
    pub pos: BlockPos,
    pub is_front_text: bool,
    #[serde(default)]
    pub lines: [String; 4],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filtered_lines: Option<[String; 4]>,
}

impl OpenSignEditorState {
    /// The screen's immutable preload snapshot selected by the startup client
    /// text-filter setting. Missing filtered NBT falls back to raw messages.
    pub fn lines_for_filter(&self, should_filter: bool) -> &[String; 4] {
        if should_filter {
            self.filtered_lines.as_ref().unwrap_or(&self.lines)
        } else {
            &self.lines
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GhostRecipeState {
    pub container_id: i32,
    pub recipe_display_type_id: i32,
    pub recipe_display_type: String,
    pub recipe_display_body_len: usize,
    #[serde(default)]
    pub recipe_display: Option<ProtocolRecipeDisplaySummary>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub slots: Vec<GhostRecipeSlotState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GhostRecipeSlotState {
    pub slot_id: i16,
    pub items: Vec<ProtocolItemStackSummary>,
    pub is_result: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PongResponseState {
    pub time: i64,
}

impl WorldStore {
    pub fn apply_low_disk_space_warning(&mut self) {
        self.counters.low_disk_space_warnings += 1;
        self.client_ui.low_disk_space_warning_count += 1;
    }

    pub fn apply_clear_dialog(&mut self) {
        self.counters.clear_dialog_packets += 1;
        self.client_ui.clear_dialog();
    }

    /// Closes the active client dialog without recording a server packet and
    /// restores the native world-owned screen that preceded it.
    pub fn close_dialog(&mut self) -> bool {
        self.client_ui.clear_dialog()
    }

    pub fn wait_for_dialog_response(&mut self) -> bool {
        self.client_ui.wait_for_dialog_response()
    }

    /// Opens a dialog from a client-side dialog action without recording a
    /// clientbound ShowDialog packet. Replacing an active dialog preserves the
    /// original non-dialog previous screen, matching vanilla's dialog stack.
    pub fn show_dialog_from_action(&mut self, dialog: DialogState) -> bool {
        self.client_ui.show_dialog(dialog);
        true
    }

    /// Selects the custom addition installed by Vanilla `PauseScreen.init`.
    ///
    /// One tagged dialog is used directly, multiple tagged dialogs route
    /// through the built-in custom-options list, and a non-empty server-links
    /// packet is the final fallback. Invalid synchronized dialog data fails
    /// closed instead of manufacturing a local replacement.
    pub fn pause_screen_custom_dialog(&self) -> Option<PauseScreenCustomDialogState> {
        if self
            .registry_unresolved_holder_ids_for_tag(DIALOG_REGISTRY, PAUSE_SCREEN_ADDITIONS_TAG)
            .is_some_and(|holders| !holders.is_empty())
        {
            return None;
        }
        let tagged = self
            .registry_resource_ids_for_tag(DIALOG_REGISTRY, PAUSE_SCREEN_ADDITIONS_TAG)
            .ok()?
            .unwrap_or_default();
        let resource_id = match tagged.as_slice() {
            [] if !self.server_links().is_empty() => SERVER_LINKS_DIALOG,
            [] => return None,
            [resource_id] => *resource_id,
            _ => CUSTOM_OPTIONS_DIALOG,
        };
        let entry = self
            .frozen_registry_access()
            .entry_by_resource_id(DIALOG_REGISTRY, resource_id)?;
        let dialog = self.dialog_state_from_reference(entry.holder_id);
        let data = dialog.data.as_ref()?;
        let common = common_dialog_data(data);
        let external_title = common
            .external_title
            .clone()
            .unwrap_or_else(|| common.title.clone());
        Some(PauseScreenCustomDialogState {
            resource_id: entry.resource_id.to_string(),
            external_title,
            dialog,
        })
    }

    /// Opens the frozen registry dialog selected when a native pause menu was
    /// initialized, preserving the existing dialog parent/return lifecycle.
    pub fn open_pause_screen_custom_dialog(
        &mut self,
        custom: &PauseScreenCustomDialogState,
    ) -> bool {
        self.client_ui.show_dialog(custom.dialog.clone());
        true
    }

    /// Decodes and opens a ShowDialog click event with the registry context of
    /// its containing dialog, without recording a clientbound packet.
    pub fn show_dialog_from_component_click(&mut self, dialog: &NbtSummaryValue) -> bool {
        let context_free = self
            .current_dialog()
            .is_some_and(|current| current.holder_kind == "context_free");
        let target =
            decode_component_click_dialog(dialog, (!context_free).then_some(&self.registries));
        let Ok(target) = target else {
            return false;
        };
        self.client_ui.show_dialog(target);
        true
    }

    pub fn set_dialog_link_options(&mut self, enabled: bool, prompt: bool) {
        self.client_ui.dialog_chat_links_disabled = !enabled;
        self.client_ui.dialog_chat_links_prompt_disabled = !prompt;
    }

    pub fn open_dialog_url(&mut self, url: String, after_action: String) -> bool {
        if self.client_ui.dialog_chat_links_disabled || self.current_dialog().is_none() {
            return false;
        }
        self.open_url_with_after_action(url, after_action)
    }

    /// Applies `Screen.clickUrlAction` from a non-dialog screen such as
    /// ChatScreen. The same startup chat-link options and confirmation owner
    /// are reused, but no active custom dialog is required.
    pub fn open_chat_url(&mut self, url: String) -> bool {
        if self.client_ui.dialog_chat_links_disabled {
            return false;
        }
        self.open_url_with_after_action(url, "none".to_string())
    }

    fn open_url_with_after_action(&mut self, url: String, after_action: String) -> bool {
        if self.client_ui.dialog_chat_links_prompt_disabled {
            self.client_ui.dialog_open_url_requests.push(url);
            return true;
        }
        self.client_ui.dialog_warning_open = false;
        self.client_ui.dialog_warning_focused_button = None;
        self.client_ui.dialog_link_confirmation =
            Some(DialogLinkConfirmationState { url, after_action });
        self.client_ui.dialog_link_focused_button = None;
        self.client_ui.advance_dialog_screen_revision();
        true
    }

    pub fn dialog_link_confirmation(&self) -> Option<&DialogLinkConfirmationState> {
        if self.client_ui.dialog_warning_open {
            None
        } else {
            self.client_ui.dialog_link_confirmation.as_ref()
        }
    }

    pub fn dialog_link_focused_button(&self) -> Option<usize> {
        self.client_ui.dialog_link_focused_button
    }

    pub fn focus_dialog_link_button(&mut self, index: usize) -> bool {
        if self.dialog_link_confirmation().is_none() || index >= 3 {
            return false;
        }
        self.client_ui.dialog_link_focused_button = Some(index);
        true
    }

    pub fn resolve_dialog_link_confirmation(&mut self, open: bool) -> bool {
        if self.client_ui.dialog_warning_open {
            return false;
        }
        let Some(confirmation) = self.client_ui.dialog_link_confirmation.take() else {
            return false;
        };
        self.client_ui.dialog_link_focused_button = None;
        if open {
            self.client_ui
                .dialog_open_url_requests
                .push(confirmation.url);
        }
        match confirmation.after_action.as_str() {
            "close" => self.client_ui.clear_dialog(),
            "none" => {
                self.client_ui.advance_dialog_screen_revision();
                true
            }
            "wait_for_response" => self.client_ui.wait_for_dialog_response(),
            _ => false,
        }
    }

    pub fn take_dialog_open_url_requests(&mut self) -> Vec<String> {
        std::mem::take(&mut self.client_ui.dialog_open_url_requests)
    }

    /// Opens one generated DialogList button target while preserving whether
    /// its containing dialog was decoded with registry or context-free rules.
    pub fn show_dialog_list_entry(&mut self, index: usize) -> bool {
        let Some((context_free, holder_kind, registry_id, dialog)) =
            self.current_dialog().and_then(|current| {
                let DialogDataState::DialogList { dialogs, .. } = current.data.as_ref()? else {
                    return None;
                };
                let entry = dialogs.entries.get(index)?;
                Some((
                    current.holder_kind == "context_free",
                    entry.holder_kind.clone(),
                    entry.registry_id,
                    entry.dialog.clone(),
                ))
            })
        else {
            return false;
        };
        let target = match holder_kind.as_str() {
            "reference" => {
                let Some(registry_id) = registry_id else {
                    return false;
                };
                self.dialog_state_from_reference(registry_id)
            }
            "direct" => {
                let Some(dialog) = dialog else {
                    return false;
                };
                if context_free {
                    DialogState::context_free(dialog)
                } else {
                    DialogState::direct(dialog, &self.registries)
                }
            }
            _ => return false,
        };
        if target.data.is_none() {
            return false;
        }
        self.client_ui.show_dialog(target);
        true
    }

    pub fn close_dialog_waiting_response(&mut self) -> bool {
        self.client_ui.close_dialog_waiting_response()
    }

    pub fn advance_dialog_waiting_ticks(&mut self, ticks: u32) {
        let Some(waiting_ticks) = &mut self.client_ui.dialog_waiting_ticks else {
            return;
        };
        *waiting_ticks = waiting_ticks.saturating_add(ticks);
    }

    pub fn apply_show_dialog(&mut self, packet: ProtocolShowDialog) {
        self.counters.show_dialog_packets += 1;
        let dialog = self.dialog_state_from_packet(packet);
        self.client_ui.show_dialog(dialog);
    }

    pub fn apply_code_of_conduct(&mut self, text: String) {
        self.counters.code_of_conduct_packets += 1;
        self.counters.last_code_of_conduct_len = text.len();
        let text_hash = code_of_conduct_text_hash(&text);
        self.client_ui.last_code_of_conduct = Some(CodeOfConductState { text, text_hash });
    }

    pub fn apply_mount_screen_open(&mut self, packet: ProtocolMountScreenOpen) {
        self.counters.mount_screen_open_packets += 1;
        let Some(entity_type_id) = self.entity_type_id(packet.entity_id) else {
            return;
        };
        if !is_vanilla_abstract_horse_type(entity_type_id)
            && !is_vanilla_abstract_nautilus_type(entity_type_id)
        {
            return;
        }
        self.client_ui.current_book = None;
        self.client_ui.current_advancements_screen = false;
        let mount = MountScreenState {
            container_id: packet.container_id,
            inventory_columns: packet.inventory_columns,
            entity_id: packet.entity_id,
        };
        self.client_ui.last_mount_screen = Some(mount);
        self.apply_mount_screen_open_container(mount);
    }

    pub fn apply_open_sign_editor(&mut self, packet: ProtocolOpenSignEditor) {
        self.counters.open_sign_editor_packets += 1;
        let pos = protocol_block_pos(packet.pos);
        if !self.has_sign_block_entity_at(pos) {
            self.counters.open_sign_editor_updates_ignored += 1;
            return;
        }
        let (lines, filtered_lines) = self
            .sign_text_state_at(pos)
            .map(|text| {
                let side = text.side(packet.is_front_text);
                (side.plain_lines(), side.filtered_plain_lines())
            })
            .unwrap_or_default();
        self.counters.open_sign_editor_updates_applied += 1;
        self.client_ui.replace_screen();
        self.client_ui.last_open_sign_editor = Some(OpenSignEditorState {
            pos,
            is_front_text: packet.is_front_text,
            lines,
            filtered_lines,
        });
    }

    pub fn apply_place_ghost_recipe(&mut self, packet: ProtocolPlaceGhostRecipe) {
        self.counters.ghost_recipe_packets += 1;
        let Some(context) = self
            .recipe_update_listener_context()
            .filter(|context| context.container_id == packet.container_id)
        else {
            self.counters.ghost_recipe_updates_ignored += 1;
            return;
        };
        let recipe_display = packet.recipe_display;
        let Some(displays) = recipe_display.slot_displays() else {
            self.counters.ghost_recipe_updates_ignored += 1;
            return;
        };
        if !displays
            .into_iter()
            .all(|display| slot_display_holders_are_resolved(self, display))
        {
            self.counters.ghost_recipe_updates_ignored += 1;
            return;
        }
        let Some(slots) = self.resolve_ghost_recipe_slots(&recipe_display, context) else {
            self.counters.ghost_recipe_updates_ignored += 1;
            return;
        };
        self.counters.ghost_recipe_updates_applied += 1;
        self.client_ui.last_ghost_recipe = Some(GhostRecipeState {
            container_id: packet.container_id,
            recipe_display_type_id: recipe_display.display_type.id(),
            recipe_display_type: recipe_display.display_type.as_str().to_string(),
            recipe_display_body_len: recipe_display.raw_body.len(),
            recipe_display: Some(recipe_display),
            slots,
        });
    }

    fn resolve_ghost_recipe_slots(
        &self,
        display: &ProtocolRecipeDisplaySummary,
        context: RecipeUpdateListenerContext,
    ) -> Option<Vec<GhostRecipeSlotState>> {
        let result = display.result()?;
        let mut assignments = Vec::new();
        match context.layout {
            RecipeUpdateListenerLayout::Crafting {
                grid_width,
                grid_height,
            } => {
                assignments.push((0, result, true));
                match display.crafting.as_deref() {
                    Some(CraftingRecipeDisplaySummary::Shapeless { ingredients, .. }) => {
                        let slot_count = ingredients
                            .len()
                            .min(usize::try_from(grid_width * grid_height).ok()?);
                        for (index, ingredient) in ingredients.iter().take(slot_count).enumerate() {
                            assignments.push((
                                i16::try_from(index).ok()?.checked_add(1)?,
                                ingredient,
                                false,
                            ));
                        }
                    }
                    Some(CraftingRecipeDisplaySummary::Shaped {
                        width,
                        height,
                        ingredients,
                        ..
                    }) => {
                        for_each_shaped_recipe_grid_slot(
                            grid_width,
                            grid_height,
                            *width,
                            *height,
                            ingredients,
                            |ingredient, grid_index, _, _| {
                                if let Some(slot_id) = i16::try_from(grid_index)
                                    .ok()
                                    .and_then(|slot| slot.checked_add(1))
                                {
                                    assignments.push((slot_id, ingredient, false));
                                }
                            },
                        );
                    }
                    None => {}
                }
            }
            RecipeUpdateListenerLayout::Furnace { fuel_slot_empty } => {
                assignments.push((2, result, true));
                if let Some(furnace) = display.furnace.as_ref() {
                    assignments.push((0, &furnace.ingredient, false));
                    if fuel_slot_empty {
                        assignments.push((1, &furnace.fuel, false));
                    }
                }
            }
        }

        let mut slots = Vec::with_capacity(assignments.len());
        let mut candidate_count = 0usize;
        for (slot_id, display, is_result) in assignments {
            let items = self.resolve_slot_display_stacks(display).ok()?;
            candidate_count = candidate_count.checked_add(items.len())?;
            if candidate_count > MAX_GHOST_RECIPE_CANDIDATES {
                return None;
            }
            if !items.is_empty() {
                slots.push(GhostRecipeSlotState {
                    slot_id,
                    items,
                    is_result,
                });
            }
        }
        Some(slots)
    }

    pub fn clear_ghost_recipe(&mut self) -> bool {
        self.client_ui.last_ghost_recipe.take().is_some()
    }

    pub fn apply_pong_response(&mut self, packet: ProtocolPongResponse) {
        self.counters.pong_response_packets += 1;
        self.client_ui.last_pong_response = Some(PongResponseState { time: packet.time });
    }

    pub fn client_ui(&self) -> &ClientUiState {
        &self.client_ui
    }

    pub fn game_event_screen(&self) -> Option<ClientGameEventScreenState> {
        self.client_ui.current_game_event_screen
    }

    pub(crate) fn open_game_event_screen(
        &mut self,
        screen: impl FnOnce(u64) -> ClientGameEventScreenState,
    ) -> ClientGameEventScreenState {
        self.client_ui.replace_screen();
        self.client_ui.game_event_screen_generation = self
            .client_ui
            .game_event_screen_generation
            .saturating_add(1);
        let screen = screen(self.client_ui.game_event_screen_generation);
        self.client_ui.current_game_event_screen = Some(screen);
        screen
    }

    pub fn close_game_event_screen(&mut self, generation: u64) -> bool {
        if self
            .client_ui
            .current_game_event_screen
            .is_none_or(|screen| screen.generation() != generation)
        {
            return false;
        }
        self.client_ui.current_game_event_screen = None;
        true
    }

    pub fn clear_game_event_screen(&mut self) -> bool {
        self.client_ui.current_game_event_screen.take().is_some()
    }

    pub fn current_dialog(&self) -> Option<&DialogState> {
        self.client_ui.current_dialog.as_ref()
    }

    pub fn current_dialog_input_values(&self) -> &[Option<DialogInputValueState>] {
        &self.client_ui.dialog_input_values
    }

    pub fn toggle_dialog_boolean_input(&mut self, index: usize) -> bool {
        let Some(Some(DialogInputValueState::Boolean { selected })) =
            self.client_ui.dialog_input_values.get_mut(index)
        else {
            return false;
        };
        *selected = !*selected;
        true
    }

    pub fn set_dialog_number_range_slider_value(&mut self, index: usize, value: f64) -> bool {
        let Some(Some(DialogInputValueState::NumberRange { slider_value_bits })) =
            self.client_ui.dialog_input_values.get_mut(index)
        else {
            return false;
        };
        let value = if value < 0.0 {
            0.0
        } else if value > 1.0 {
            1.0
        } else {
            value
        };
        *slider_value_bits = value.to_bits();
        true
    }

    pub fn begin_dialog_number_range_drag(&mut self, index: usize) -> bool {
        if !matches!(
            self.client_ui.dialog_input_values.get(index),
            Some(Some(DialogInputValueState::NumberRange { .. }))
        ) {
            return false;
        }
        self.client_ui.dialog_dragging_number_range_input = Some(index);
        true
    }

    pub fn end_dialog_number_range_drag(&mut self) -> Option<usize> {
        self.client_ui.dialog_dragging_number_range_input.take()
    }

    pub fn dialog_dragging_number_range_input(&self) -> Option<usize> {
        self.client_ui.dialog_dragging_number_range_input
    }

    pub fn set_dialog_single_option_index(&mut self, index: usize, selected_index: usize) -> bool {
        if !self
            .current_dialog()
            .and_then(|dialog| dialog.data.as_ref())
            .map(common_dialog_data)
            .and_then(|common| common.input_data.get(index))
            .is_some_and(|input| {
                matches!(
                    &input.control,
                    DialogInputControlState::SingleOption { options, .. }
                        if selected_index < options.len()
                )
            })
        {
            return false;
        }
        let Some(Some(DialogInputValueState::SingleOption {
            selected_index: current,
        })) = self.client_ui.dialog_input_values.get_mut(index)
        else {
            return false;
        };
        *current = selected_index;
        true
    }

    pub fn dialog_text_edit_state(&self) -> Option<DialogTextEditState> {
        self.client_ui.dialog_text_edit.filter(|edit| {
            self.client_ui.dialog_focus
                == Some(DialogFocusState::Input {
                    index: edit.input_index,
                })
        })
    }

    pub fn dialog_text_edit_state_for_input(&self, index: usize) -> Option<DialogTextEditState> {
        self.client_ui
            .dialog_text_edit
            .filter(|edit| edit.input_index == index)
    }

    pub fn focus_dialog_text_input(
        &mut self,
        index: usize,
        cursor: usize,
        extend_selection: bool,
    ) -> bool {
        let Some(Some(DialogInputValueState::Text { value })) =
            self.client_ui.dialog_input_values.get(index)
        else {
            return false;
        };
        if self.dialog_text_max_length(index).is_none() {
            return false;
        }
        let cursor = cursor.min(value.chars().count());
        let previous = self
            .client_ui
            .dialog_text_edit
            .filter(|edit| edit.input_index == index);
        let selection = previous
            .filter(|_| extend_selection)
            .map_or(cursor, |edit| edit.selection);
        let scroll_amount = previous.map_or_else(
            || {
                if self.dialog_multiline_text_settings(index).is_some() {
                    i32::MAX
                } else {
                    0
                }
            },
            |edit| edit.scroll_amount,
        );
        self.client_ui.dialog_text_edit = Some(DialogTextEditState {
            input_index: index,
            cursor,
            selection,
            scroll_amount,
        });
        self.client_ui.dialog_focus = Some(DialogFocusState::Input { index });
        self.client_ui.dialog_number_range_keyboard_input = None;
        self.client_ui.dialog_dragging_multiline_scroll_input = None;
        self.client_ui.dialog_dragging_multiline_scroll_y = None;
        true
    }

    pub fn dialog_focused_input(&self) -> Option<usize> {
        match self.client_ui.dialog_focus {
            Some(DialogFocusState::Input { index }) => Some(index),
            _ => None,
        }
    }

    pub fn dialog_focused_body(&self) -> Option<usize> {
        match self.client_ui.dialog_focus {
            Some(DialogFocusState::Body { index, .. }) => Some(index),
            _ => None,
        }
    }

    pub fn dialog_focus(&self) -> Option<DialogFocusState> {
        self.client_ui.dialog_focus
    }

    pub fn dialog_body_scroll_amount(&self) -> f64 {
        let amount = f64::from_bits(self.client_ui.dialog_body_scroll_amount_bits);
        if amount.is_finite() {
            amount.max(0.0)
        } else {
            0.0
        }
    }

    pub fn set_dialog_body_scroll_amount(&mut self, amount: f64, max_scroll: f64) -> bool {
        let max_scroll = if max_scroll.is_finite() {
            max_scroll.max(0.0)
        } else {
            0.0
        };
        let amount = if amount.is_finite() {
            amount.clamp(0.0, max_scroll)
        } else {
            0.0
        };
        let bits = amount.to_bits();
        if self.client_ui.dialog_body_scroll_amount_bits == bits {
            return false;
        }
        self.client_ui.dialog_body_scroll_amount_bits = bits;
        true
    }

    pub fn begin_dialog_body_scroll_drag(&mut self, cursor_y: i32) {
        self.client_ui.dialog_body_scroll_drag_y = Some(cursor_y);
    }

    pub fn dialog_body_scroll_drag_y(&self) -> Option<i32> {
        self.client_ui.dialog_body_scroll_drag_y
    }

    pub fn update_dialog_body_scroll_drag_y(&mut self, cursor_y: i32) -> Option<i32> {
        let previous = self.client_ui.dialog_body_scroll_drag_y.replace(cursor_y)?;
        Some(cursor_y.saturating_sub(previous))
    }

    pub fn end_dialog_body_scroll_drag(&mut self) -> bool {
        self.client_ui.dialog_body_scroll_drag_y.take().is_some()
    }

    pub fn focus_dialog_body_item(&mut self, index: usize) -> bool {
        if !self
            .current_dialog()
            .and_then(|dialog| dialog.data.as_ref())
            .map(common_dialog_data)
            .and_then(|common| common.body_data.get(index))
            .is_some_and(|body| matches!(body, DialogBodyState::Item { .. }))
        {
            return false;
        }
        self.client_ui.dialog_focus = Some(DialogFocusState::Body {
            index,
            part: DialogBodyFocusPartState::Primary,
        });
        self.client_ui.dialog_number_range_keyboard_input = None;
        self.client_ui.dialog_dragging_text_input = None;
        self.client_ui.dialog_dragging_multiline_scroll_input = None;
        self.client_ui.dialog_dragging_multiline_scroll_y = None;
        true
    }

    pub fn focus_dialog_navigation_target(
        &mut self,
        target: DialogFocusState,
        number_range_can_change: bool,
    ) -> bool {
        let server_link_count = self.server_links().len();
        let Some((valid, is_number_range, is_text, is_multiline_text)) = self
            .current_dialog()
            .and_then(|dialog| dialog.data.as_ref())
            .map(|data| {
                let common = common_dialog_data(data);
                let valid = match target {
                    DialogFocusState::WarningButton => true,
                    DialogFocusState::Body { index, part } => common
                        .body_data
                        .get(index)
                        .is_some_and(|body| match (body, part) {
                            (_, DialogBodyFocusPartState::Primary) => true,
                            (
                                DialogBodyState::Item {
                                    description: Some(_),
                                    ..
                                },
                                DialogBodyFocusPartState::Description,
                            ) => true,
                            _ => false,
                        }),
                    DialogFocusState::Input { index } => common.input_data.get(index).is_some(),
                    DialogFocusState::Action { index } => match data {
                        DialogDataState::Notice { .. } => index == 0,
                        DialogDataState::Confirmation { .. } => index < 2,
                        DialogDataState::MultiAction {
                            actions,
                            exit_action,
                            ..
                        } => {
                            index
                                < actions
                                    .len()
                                    .saturating_add(usize::from(exit_action.is_some()))
                        }
                        DialogDataState::DialogList {
                            dialogs,
                            exit_action,
                            ..
                        } => {
                            index
                                < dialogs
                                    .entries
                                    .len()
                                    .saturating_add(usize::from(exit_action.is_some()))
                        }
                        DialogDataState::ServerLinks { exit_action, .. } => {
                            index
                                < server_link_count
                                    .saturating_add(usize::from(exit_action.is_some()))
                        }
                    },
                };
                let control = match target {
                    DialogFocusState::Input { index } => {
                        common.input_data.get(index).map(|input| &input.control)
                    }
                    _ => None,
                };
                (
                    valid,
                    matches!(control, Some(DialogInputControlState::NumberRange { .. })),
                    matches!(control, Some(DialogInputControlState::Text { .. })),
                    matches!(
                        control,
                        Some(DialogInputControlState::Text {
                            multiline: Some(_),
                            ..
                        })
                    ),
                )
            })
        else {
            return false;
        };
        if !valid {
            return false;
        }

        self.client_ui.dialog_focus = Some(target);
        self.client_ui.dialog_dragging_text_input = None;
        self.client_ui.dialog_dragging_multiline_scroll_input = None;
        self.client_ui.dialog_dragging_multiline_scroll_y = None;
        self.client_ui.dialog_number_range_keyboard_input = match target {
            DialogFocusState::Input { index } if number_range_can_change && is_number_range => {
                Some(index)
            }
            _ => None,
        };
        if let DialogFocusState::Input { index } = target {
            if is_text
                && !self
                    .client_ui
                    .dialog_text_edit
                    .is_some_and(|edit| edit.input_index == index)
            {
                let cursor = match self.client_ui.dialog_input_values.get(index) {
                    Some(Some(DialogInputValueState::Text { value })) => value.chars().count(),
                    _ => 0,
                };
                self.client_ui.dialog_text_edit = Some(DialogTextEditState {
                    input_index: index,
                    cursor,
                    selection: cursor,
                    scroll_amount: if is_multiline_text { i32::MAX } else { 0 },
                });
            }
        }
        true
    }

    pub fn focus_dialog_mouse_input(&mut self, index: usize) -> bool {
        let control = self
            .current_dialog()
            .and_then(|dialog| dialog.data.as_ref())
            .map(common_dialog_data)
            .and_then(|common| common.input_data.get(index))
            .map(|input| &input.control);
        let number_range = matches!(control, Some(DialogInputControlState::NumberRange { .. }));
        if !matches!(
            control,
            Some(
                DialogInputControlState::Boolean { .. }
                    | DialogInputControlState::NumberRange { .. }
                    | DialogInputControlState::SingleOption { .. }
            )
        ) {
            return false;
        }
        self.client_ui.dialog_focus = Some(DialogFocusState::Input { index });
        self.client_ui.dialog_number_range_keyboard_input = number_range.then_some(index);
        self.client_ui.dialog_dragging_text_input = None;
        self.client_ui.dialog_dragging_multiline_scroll_input = None;
        self.client_ui.dialog_dragging_multiline_scroll_y = None;
        true
    }

    pub fn clear_dialog_focus(&mut self) -> bool {
        self.client_ui.dialog_number_range_keyboard_input = None;
        self.client_ui.dialog_dragging_text_input = None;
        self.client_ui.dialog_dragging_multiline_scroll_input = None;
        self.client_ui.dialog_dragging_multiline_scroll_y = None;
        self.client_ui.dialog_focus.take().is_some()
    }

    pub fn dialog_number_range_keyboard_can_change(&self, index: usize) -> bool {
        self.dialog_focused_input() == Some(index)
            && self.client_ui.dialog_number_range_keyboard_input == Some(index)
    }

    pub fn toggle_dialog_number_range_keyboard_change(&mut self, index: usize) -> bool {
        if self.dialog_focused_input() != Some(index)
            || !matches!(
                self.client_ui.dialog_input_values.get(index),
                Some(Some(DialogInputValueState::NumberRange { .. }))
            )
        {
            return false;
        }
        if self.client_ui.dialog_number_range_keyboard_input == Some(index) {
            self.client_ui.dialog_number_range_keyboard_input = None;
        } else {
            self.client_ui.dialog_number_range_keyboard_input = Some(index);
        }
        true
    }

    pub fn begin_dialog_text_input_drag(&mut self, index: usize) -> bool {
        if !self
            .client_ui
            .dialog_text_edit
            .is_some_and(|edit| edit.input_index == index)
        {
            return false;
        }
        self.client_ui.dialog_dragging_text_input = Some(index);
        true
    }

    pub fn end_dialog_text_input_drag(&mut self) -> Option<usize> {
        self.client_ui.dialog_dragging_text_input.take()
    }

    pub fn dialog_dragging_text_input(&self) -> Option<usize> {
        self.client_ui.dialog_dragging_text_input
    }

    pub fn set_dialog_text_input_value_and_selection(
        &mut self,
        index: usize,
        value: String,
        cursor: usize,
        selection: usize,
    ) -> bool {
        let Some(max_length) = self.dialog_text_max_length(index) else {
            return false;
        };
        let text_len = value.chars().count();
        if value.encode_utf16().count() > max_length
            || cursor > text_len
            || selection > text_len
            || !self
                .client_ui
                .dialog_text_edit
                .is_some_and(|edit| edit.input_index == index)
            || self.dialog_focused_input() != Some(index)
        {
            return false;
        }
        let Some(Some(DialogInputValueState::Text { value: current })) =
            self.client_ui.dialog_input_values.get_mut(index)
        else {
            return false;
        };
        *current = value;
        let scroll_amount = self
            .client_ui
            .dialog_text_edit
            .map_or(0, |edit| edit.scroll_amount);
        self.client_ui.dialog_text_edit = Some(DialogTextEditState {
            input_index: index,
            cursor,
            selection,
            scroll_amount,
        });
        true
    }

    fn dialog_text_max_length(&self, index: usize) -> Option<usize> {
        let input = self
            .current_dialog()?
            .data
            .as_ref()
            .map(common_dialog_data)?
            .input_data
            .get(index)?;
        match &input.control {
            DialogInputControlState::Text { max_length, .. } => usize::try_from(*max_length).ok(),
            _ => None,
        }
    }

    pub fn dialog_multiline_text_settings(
        &self,
        index: usize,
    ) -> Option<(Option<i32>, Option<i32>)> {
        let input = self
            .current_dialog()?
            .data
            .as_ref()
            .map(common_dialog_data)?
            .input_data
            .get(index)?;
        match &input.control {
            DialogInputControlState::Text {
                multiline: Some(multiline),
                ..
            } => Some((multiline.max_lines, multiline.height)),
            _ => None,
        }
    }

    pub fn set_dialog_multiline_text_scroll_amount(
        &mut self,
        index: usize,
        amount: i32,
        max_scroll: i32,
    ) -> bool {
        if self.dialog_multiline_text_settings(index).is_none()
            || self.dialog_focused_input() != Some(index)
        {
            return false;
        }
        let Some(edit) = self.client_ui.dialog_text_edit.as_mut() else {
            return false;
        };
        if edit.input_index != index {
            return false;
        }
        let amount = amount.clamp(0, max_scroll.max(0));
        if edit.scroll_amount == amount {
            return false;
        }
        edit.scroll_amount = amount;
        true
    }

    pub fn begin_dialog_multiline_scroll_drag(&mut self, index: usize, cursor_y: i32) -> bool {
        if self.dialog_multiline_text_settings(index).is_none()
            || !self
                .client_ui
                .dialog_text_edit
                .is_some_and(|edit| edit.input_index == index)
        {
            return false;
        }
        self.client_ui.dialog_dragging_text_input = None;
        self.client_ui.dialog_dragging_multiline_scroll_input = Some(index);
        self.client_ui.dialog_dragging_multiline_scroll_y = Some(cursor_y);
        true
    }

    pub fn dialog_dragging_multiline_scroll_input(&self) -> Option<usize> {
        self.client_ui.dialog_dragging_multiline_scroll_input
    }

    pub fn update_dialog_multiline_scroll_drag_y(&mut self, cursor_y: i32) -> Option<i32> {
        let previous = self
            .client_ui
            .dialog_dragging_multiline_scroll_y
            .replace(cursor_y)?;
        Some(cursor_y.saturating_sub(previous))
    }

    pub fn end_dialog_multiline_scroll_drag(&mut self) -> Option<usize> {
        self.client_ui.dialog_dragging_multiline_scroll_y = None;
        self.client_ui.dialog_dragging_multiline_scroll_input.take()
    }

    pub fn dialog_waiting_ticks(&self) -> Option<u32> {
        self.client_ui.dialog_waiting_ticks
    }

    pub fn dialog_warning_is_open(&self) -> bool {
        self.client_ui.dialog_warning_open
    }

    /// Monotonic identity for the currently owned dialog, warning, or waiting
    /// screen. Platform input uses it to keep vanilla's double-click timing
    /// scoped to one concrete Screen instance.
    pub fn dialog_screen_revision(&self) -> u64 {
        self.client_ui.dialog_screen_revision
    }

    pub fn open_dialog_warning(&mut self) -> bool {
        if self.current_dialog().is_none() || self.client_ui.dialog_warning_open {
            return false;
        }
        self.client_ui.dialog_warning_open = true;
        self.client_ui.dialog_warning_focused_button = None;
        self.client_ui.advance_dialog_screen_revision();
        true
    }

    pub fn close_dialog_warning(&mut self) -> bool {
        self.client_ui.dialog_warning_focused_button = None;
        let closed = std::mem::take(&mut self.client_ui.dialog_warning_open);
        if closed {
            self.client_ui.advance_dialog_screen_revision();
        }
        closed
    }

    pub fn dialog_warning_focused_button(&self) -> Option<bool> {
        self.client_ui.dialog_warning_focused_button
    }

    pub fn focus_dialog_warning_button(&mut self, disconnect: bool) -> bool {
        if !self.client_ui.dialog_warning_open {
            return false;
        }
        self.client_ui.dialog_warning_focused_button = Some(disconnect);
        true
    }

    pub fn dialog_screen_is_open(&self) -> bool {
        self.dialog_link_confirmation().is_some()
            || self.dialog_warning_is_open()
            || self.current_dialog().is_some()
            || self.dialog_waiting_ticks().is_some()
    }

    pub fn dialog_waiting_can_close(&self) -> bool {
        self.dialog_waiting_ticks()
            .is_some_and(|ticks| ticks >= DIALOG_WAITING_CLOSE_ACTIVE_TICKS)
    }

    pub fn advancements_screen_is_open(&self) -> bool {
        self.client_ui.current_advancements_screen
    }

    pub fn open_advancements_screen(&mut self) -> bool {
        let was_open = self.client_ui.current_advancements_screen;
        self.client_ui.replace_screen();
        self.client_ui.current_advancements_screen = true;
        !was_open
    }

    pub fn close_advancements_screen(&mut self) -> bool {
        let was_open = self.client_ui.current_advancements_screen;
        self.client_ui.current_advancements_screen = false;
        was_open
    }

    pub fn stats_screen_is_open(&self) -> bool {
        self.client_ui.current_stats_screen
    }

    pub fn open_stats_screen(&mut self) -> bool {
        let was_open = self.client_ui.current_stats_screen;
        self.client_ui.replace_screen();
        self.client_ui.current_stats_screen = true;
        self.client_ui.stats_screen_loading = true;
        !was_open
    }

    pub fn close_stats_screen(&mut self) -> bool {
        let was_open = self.client_ui.current_stats_screen;
        self.client_ui.current_stats_screen = false;
        was_open
    }

    pub fn stats_screen_is_loading(&self) -> bool {
        self.client_ui.current_stats_screen && self.client_ui.stats_screen_loading
    }

    pub fn last_code_of_conduct(&self) -> Option<&CodeOfConductState> {
        self.client_ui.last_code_of_conduct.as_ref()
    }

    pub fn low_disk_space_warning_count(&self) -> usize {
        self.client_ui.low_disk_space_warning_count
    }

    pub fn last_mount_screen(&self) -> Option<&MountScreenState> {
        self.client_ui.last_mount_screen.as_ref()
    }

    pub fn last_open_sign_editor(&self) -> Option<&OpenSignEditorState> {
        self.client_ui.last_open_sign_editor.as_ref()
    }

    pub fn last_ghost_recipe(&self) -> Option<&GhostRecipeState> {
        self.client_ui.last_ghost_recipe.as_ref()
    }

    pub fn last_pong_response(&self) -> Option<&PongResponseState> {
        self.client_ui.last_pong_response.as_ref()
    }

    fn dialog_state_from_packet(&self, packet: ProtocolShowDialog) -> DialogState {
        match packet.dialog {
            DialogHolder::Reference { registry_id } => {
                self.dialog_state_from_reference(registry_id)
            }
            DialogHolder::Direct { dialog } => DialogState::direct(dialog, &self.registries),
            DialogHolder::ContextFree { dialog } => DialogState::context_free(dialog),
        }
    }

    fn dialog_state_from_reference(&self, registry_id: i32) -> DialogState {
        let Some(entry) = self
            .frozen_registry_access()
            .entry_by_holder_id(DIALOG_REGISTRY, registry_id)
        else {
            return DialogState::unresolved_reference(
                registry_id,
                None,
                format!("missing {DIALOG_REGISTRY} holder {registry_id}"),
            );
        };
        let resource_id = entry.resource_id.to_string();
        match decode_dialog_registry_entry(entry) {
            Ok(dialog) => {
                DialogState::reference(registry_id, resource_id, dialog.clone(), &self.registries)
            }
            Err(error) => DialogState::unresolved_reference(registry_id, Some(resource_id), error),
        }
    }
}

pub fn code_of_conduct_text_hash(text: &str) -> i32 {
    java_string_hash_code(text)
}

pub fn java_string_hash_code(text: &str) -> i32 {
    text.encode_utf16().fold(0i32, |hash, unit| {
        hash.wrapping_mul(31).wrapping_add(i32::from(unit))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID;
    use bbb_protocol::codec::Encoder;
    use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID;
    use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_HORSE_ID, VANILLA_ENTITY_TYPE_PLAYER_ID};
    use bbb_protocol::menu_types::{
        VANILLA_MENU_TYPE_BLAST_FURNACE_ID, VANILLA_MENU_TYPE_CRAFTING_ID,
        VANILLA_MENU_TYPE_FURNACE_ID, VANILLA_MENU_TYPE_GENERIC_9X1_ID,
        VANILLA_MENU_TYPE_SMOKER_ID,
    };
    use bbb_protocol::packets::{
        AddEntity as ProtocolAddEntity, BlockPos as ProtocolBlockPos,
        ContainerSetSlot as ProtocolContainerSetSlot, DataComponentPatchSummary, DialogHolder,
        InteractionHand, ItemStackSummary, OpenBook as ProtocolOpenBook,
        OpenScreen as ProtocolOpenScreen, RegistryTags, ServerLinkEntry, ServerLinkKnownType,
        ServerLinkType, ServerLinks, SetPlayerInventory as ProtocolSetPlayerInventory,
        TagNetworkPayload, UpdateTags, Vec3d as ProtocolVec3d, WrittenBookContentSummary,
    };
    use bbb_protocol::recipe_wire_types::{
        VANILLA_RECIPE_DISPLAY_TYPE_STONECUTTER_ID, VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID,
        VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID,
    };
    use bbb_protocol::NbtSummaryEntry;
    use uuid::Uuid;

    #[test]
    fn ui_does_not_restore_writable_book_component_id_alias() {
        let source = include_str!("ui.rs");
        let old_alias = ["VANILLA_WRITABLE_BOOK_CONTENT_", "COMPONENT_ID"].concat();
        assert!(!source.contains(&format!("const {old_alias}")));
    }

    #[test]
    fn pause_screen_custom_dialog_selects_single_tag_and_opens_frozen_target() {
        let mut store = WorldStore::new();
        store.record_registry_entries(
            DIALOG_REGISTRY,
            0,
            vec![crate::RegistryPacketEntry::with_known_pack_data_for_test(
                "example:custom",
                br#"{"type":"minecraft:notice","title":"Internal","external_title":"External"}"#
                    .to_vec(),
            )],
        );
        apply_pause_screen_dialog_tag(&mut store, vec![0]);

        let custom = store.pause_screen_custom_dialog().expect("custom dialog");
        assert_eq!(custom.resource_id(), "example:custom");
        assert_eq!(custom.external_title().text, "External");
        assert!(store.open_pause_screen_custom_dialog(&custom));

        let current = store.current_dialog().expect("opened dialog");
        assert_eq!(current.resource_id.as_deref(), Some("example:custom"));
        assert!(matches!(
            current.data,
            Some(DialogDataState::Notice { ref common, .. })
                if common.title.text == "Internal"
        ));
        assert_eq!(store.counters().show_dialog_packets, 0);
    }

    #[test]
    fn pause_screen_custom_dialog_uses_builtin_list_for_multiple_tagged_entries() {
        let mut store = WorldStore::new();
        store.record_registry_entries(
            DIALOG_REGISTRY,
            0,
            vec![
                pause_notice_registry_entry("example:first", "First"),
                pause_notice_registry_entry("example:second", "Second"),
                crate::RegistryPacketEntry::with_known_pack_data_for_test(
                    CUSTOM_OPTIONS_DIALOG,
                    br##"{"type":"minecraft:dialog_list","title":"Custom Options","external_title":"Server Options","dialogs":"#minecraft:pause_screen_additions"}"##
                        .to_vec(),
                ),
            ],
        );
        apply_pause_screen_dialog_tag(&mut store, vec![1, 0]);

        let custom = store.pause_screen_custom_dialog().expect("custom list");
        assert_eq!(custom.resource_id(), CUSTOM_OPTIONS_DIALOG);
        assert_eq!(custom.external_title().text, "Server Options");
        assert!(matches!(
            custom.dialog.data,
            Some(DialogDataState::DialogList {
                ref dialogs, ..
            }) if dialogs
                .entries
                .iter()
                .map(|entry| entry.resource_id.as_deref())
                .eq([Some("example:second"), Some("example:first")])
        ));
    }

    #[test]
    fn pause_screen_custom_dialog_falls_back_only_for_nonempty_server_links() {
        let mut store = WorldStore::new();
        store.record_registry_entries(
            DIALOG_REGISTRY,
            0,
            vec![crate::RegistryPacketEntry::with_known_pack_data_for_test(
                SERVER_LINKS_DIALOG,
                br#"{"type":"minecraft:server_links","title":"Links","external_title":"Server Links"}"#
                    .to_vec(),
            )],
        );
        apply_pause_screen_dialog_tag(&mut store, Vec::new());
        assert_eq!(store.pause_screen_custom_dialog(), None);

        store.apply_server_links(ServerLinks {
            links: vec![ServerLinkEntry {
                link_type: ServerLinkType::Known(ServerLinkKnownType::Website),
                url: "https://example.invalid/".to_string(),
            }],
        });
        let custom = store.pause_screen_custom_dialog().expect("server links");
        assert_eq!(custom.resource_id(), SERVER_LINKS_DIALOG);
        assert_eq!(custom.external_title().text, "Server Links");
    }

    #[test]
    fn pause_screen_custom_dialog_fails_closed_for_unresolved_or_invalid_targets() {
        let mut invalid = WorldStore::new();
        invalid.record_registry_entries(
            DIALOG_REGISTRY,
            0,
            vec![crate::RegistryPacketEntry::stub("example:stub")],
        );
        apply_pause_screen_dialog_tag(&mut invalid, vec![0]);
        assert_eq!(invalid.pause_screen_custom_dialog(), None);

        let mut unresolved = WorldStore::new();
        unresolved.record_registry_entries(
            DIALOG_REGISTRY,
            0,
            vec![crate::RegistryPacketEntry::with_known_pack_data_for_test(
                SERVER_LINKS_DIALOG,
                br#"{"type":"minecraft:server_links","title":"Links"}"#.to_vec(),
            )],
        );
        unresolved.apply_server_links(ServerLinks {
            links: vec![ServerLinkEntry {
                link_type: ServerLinkType::Known(ServerLinkKnownType::Website),
                url: "https://example.invalid/".to_string(),
            }],
        });
        apply_pause_screen_dialog_tag(&mut unresolved, vec![99]);
        assert_eq!(unresolved.pause_screen_custom_dialog(), None);
    }

    #[test]
    fn tracks_client_ui_warnings_and_dialogs() {
        let mut store = WorldStore::new();

        store.apply_low_disk_space_warning();
        store.apply_show_dialog(ProtocolShowDialog {
            dialog: DialogHolder::Direct {
                dialog: NbtSummaryValue::Compound(vec![
                    NbtSummaryEntry {
                        name: "type".to_string(),
                        value: NbtSummaryValue::String("minecraft:notice".to_string()),
                    },
                    NbtSummaryEntry {
                        name: "title".to_string(),
                        value: NbtSummaryValue::String("Title".to_string()),
                    },
                ]),
            },
        });

        assert_eq!(store.low_disk_space_warning_count(), 1);
        let direct = store.current_dialog().unwrap();
        assert_eq!(direct.holder_kind, "direct");
        assert_eq!(direct.registry_id, None);
        assert!(matches!(direct.dialog, Some(NbtSummaryValue::Compound(_))));
        assert_eq!(direct.resolution_error, None);
        assert!(matches!(
            direct.data,
            Some(crate::DialogDataState::Notice { .. })
        ));
        assert_eq!(direct.semantic_error, None);

        store.record_registry_entries(
            DIALOG_REGISTRY,
            0,
            vec![crate::RegistryPacketEntry::with_raw_data(
                "example:notice",
                dialog_nbt("minecraft:notice"),
            )],
        );
        store.apply_show_dialog(ProtocolShowDialog {
            dialog: DialogHolder::Reference { registry_id: 0 },
        });

        let referenced = store.current_dialog().unwrap();
        assert_eq!(referenced.holder_kind, "reference");
        assert_eq!(referenced.registry_id, Some(0));
        assert_eq!(referenced.resource_id.as_deref(), Some("example:notice"));
        assert!(matches!(
            referenced.dialog,
            Some(NbtSummaryValue::Compound(_))
        ));
        assert_eq!(referenced.resolution_error, None);
        assert!(matches!(
            referenced.data,
            Some(crate::DialogDataState::Notice { .. })
        ));
        assert_eq!(referenced.semantic_error, None);

        store.apply_clear_dialog();

        assert_eq!(store.current_dialog(), None);
        let counters = store.counters();
        assert_eq!(counters.low_disk_space_warnings, 1);
        assert_eq!(counters.show_dialog_packets, 2);
        assert_eq!(counters.clear_dialog_packets, 1);
    }

    #[test]
    fn dialog_list_entry_opens_registry_target_without_counting_a_server_packet() {
        let mut store = WorldStore::new();
        store.record_registry_entries(
            DIALOG_REGISTRY,
            0,
            vec![crate::RegistryPacketEntry::with_raw_data(
                "example:target",
                dialog_nbt("minecraft:notice"),
            )],
        );
        store.apply_show_dialog(ProtocolShowDialog {
            dialog: DialogHolder::Direct {
                dialog: NbtSummaryValue::Compound(vec![
                    NbtSummaryEntry {
                        name: "type".to_string(),
                        value: NbtSummaryValue::String("minecraft:dialog_list".to_string()),
                    },
                    NbtSummaryEntry {
                        name: "title".to_string(),
                        value: NbtSummaryValue::String("Menu".to_string()),
                    },
                    NbtSummaryEntry {
                        name: "dialogs".to_string(),
                        value: NbtSummaryValue::String("example:target".to_string()),
                    },
                ]),
            },
        });
        assert!(store.focus_dialog_navigation_target(DialogFocusState::Action { index: 0 }, false));
        assert!(!store.show_dialog_list_entry(1));

        assert!(store.show_dialog_list_entry(0));
        let target = store.current_dialog().unwrap();
        assert_eq!(target.holder_kind, "reference");
        assert_eq!(target.registry_id, Some(0));
        assert_eq!(target.resource_id.as_deref(), Some("example:target"));
        assert!(matches!(
            target.data,
            Some(crate::DialogDataState::Notice { ref common, .. }) if common.title.text == "Title"
        ));
        assert_eq!(store.counters().show_dialog_packets, 1);
        assert_eq!(store.dialog_focus(), None);
    }

    #[test]
    fn dialog_screen_revision_tracks_concrete_screen_replacements() {
        let mut store = WorldStore::new();
        assert_eq!(store.dialog_screen_revision(), 0);
        store.apply_clear_dialog();
        assert_eq!(store.dialog_screen_revision(), 0);

        store.apply_show_dialog(ProtocolShowDialog {
            dialog: DialogHolder::Reference { registry_id: 1 },
        });
        assert_eq!(store.dialog_screen_revision(), 1);
        assert!(store.open_dialog_warning());
        assert_eq!(store.dialog_screen_revision(), 2);
        assert!(!store.open_dialog_warning());
        assert_eq!(store.dialog_screen_revision(), 2);
        assert!(store.close_dialog_warning());
        assert_eq!(store.dialog_screen_revision(), 3);
        assert!(!store.close_dialog_warning());
        assert_eq!(store.dialog_screen_revision(), 3);

        assert!(store.wait_for_dialog_response());
        assert_eq!(store.dialog_screen_revision(), 4);
        store.advance_dialog_waiting_ticks(DIALOG_WAITING_CLOSE_ACTIVE_TICKS);
        assert!(store.close_dialog_waiting_response());
        assert_eq!(store.dialog_screen_revision(), 5);
        assert!(!store.close_dialog_waiting_response());
        assert_eq!(store.dialog_screen_revision(), 5);
    }

    #[test]
    fn dialog_open_url_prompt_owns_clear_and_cancel_lifecycle() {
        let mut store = WorldStore::new();
        show_notice(&mut store, "Links");
        let revision = store.dialog_screen_revision();

        assert!(store.open_dialog_url(
            "https://example.invalid/docs".to_string(),
            "none".to_string(),
        ));
        assert_eq!(store.dialog_screen_revision(), revision + 1);
        assert_eq!(
            store.dialog_link_confirmation(),
            Some(&DialogLinkConfirmationState {
                url: "https://example.invalid/docs".to_string(),
                after_action: "none".to_string(),
            })
        );
        assert!(store.focus_dialog_link_button(1));

        store.apply_clear_dialog();
        assert_eq!(store.counters().clear_dialog_packets, 1);
        assert!(store.current_dialog().is_some());
        assert!(store.dialog_link_confirmation().is_some());
        assert_eq!(store.dialog_link_focused_button(), Some(1));
        assert_eq!(store.dialog_screen_revision(), revision + 1);

        assert!(store.resolve_dialog_link_confirmation(false));
        assert_eq!(store.dialog_screen_revision(), revision + 2);
        assert!(store.current_dialog().is_some());
        assert_eq!(store.dialog_link_confirmation(), None);
        assert!(store.take_dialog_open_url_requests().is_empty());
    }

    #[test]
    fn chat_open_url_uses_link_options_without_active_dialog() {
        let mut store = WorldStore::new();
        assert!(store.open_chat_url("https://example.invalid/chat".to_string()));
        assert_eq!(
            store.dialog_link_confirmation(),
            Some(&DialogLinkConfirmationState {
                url: "https://example.invalid/chat".to_string(),
                after_action: "none".to_string(),
            })
        );
        assert!(store.resolve_dialog_link_confirmation(true));
        assert_eq!(
            store.take_dialog_open_url_requests(),
            ["https://example.invalid/chat"]
        );

        let mut disabled = WorldStore::new();
        disabled.set_dialog_link_options(false, true);
        assert!(!disabled.open_chat_url("https://example.invalid/blocked".to_string()));
        assert!(disabled.dialog_link_confirmation().is_none());
    }

    #[test]
    fn dialog_open_url_resolution_applies_deferred_after_action() {
        let mut close_store = WorldStore::new();
        show_notice(&mut close_store, "Close");
        assert!(close_store.open_dialog_url(
            "https://example.invalid/close".to_string(),
            "close".to_string(),
        ));
        assert!(close_store.resolve_dialog_link_confirmation(true));
        assert!(close_store.current_dialog().is_none());
        assert_eq!(
            close_store.take_dialog_open_url_requests(),
            ["https://example.invalid/close"]
        );
        assert_eq!(close_store.counters().show_dialog_packets, 1);
        assert_eq!(close_store.counters().clear_dialog_packets, 0);

        let mut wait_store = WorldStore::new();
        show_notice(&mut wait_store, "Wait");
        assert!(wait_store.open_dialog_url(
            "https://example.invalid/wait".to_string(),
            "wait_for_response".to_string(),
        ));
        assert!(wait_store.resolve_dialog_link_confirmation(false));
        assert!(wait_store.current_dialog().is_none());
        assert_eq!(wait_store.dialog_waiting_ticks(), Some(0));
        assert!(wait_store.take_dialog_open_url_requests().is_empty());
    }

    #[test]
    fn server_dialogs_preserve_nested_confirm_link_return_screens() {
        let mut store = WorldStore::new();
        show_notice(&mut store, "First");
        assert!(store.open_dialog_url(
            "https://example.invalid/first".to_string(),
            "none".to_string(),
        ));
        assert!(store.focus_dialog_link_button(1));

        show_notice(&mut store, "Second");
        assert_eq!(store.dialog_link_confirmation(), None);
        assert_eq!(dialog_title(&store), Some("Second"));
        assert!(store.open_dialog_url(
            "https://example.invalid/second".to_string(),
            "none".to_string(),
        ));
        assert!(store.focus_dialog_link_button(2));

        show_notice(&mut store, "Third");
        assert_eq!(store.dialog_link_confirmation(), None);
        assert_eq!(dialog_title(&store), Some("Third"));
        store.apply_clear_dialog();
        assert_eq!(dialog_title(&store), Some("Second"));
        assert_eq!(
            store.dialog_link_confirmation(),
            Some(&DialogLinkConfirmationState {
                url: "https://example.invalid/second".to_string(),
                after_action: "none".to_string(),
            })
        );
        assert_eq!(store.dialog_link_focused_button(), Some(2));
        assert!(store.resolve_dialog_link_confirmation(false));

        store.apply_clear_dialog();
        assert_eq!(dialog_title(&store), Some("First"));
        assert_eq!(
            store.dialog_link_confirmation(),
            Some(&DialogLinkConfirmationState {
                url: "https://example.invalid/first".to_string(),
                after_action: "none".to_string(),
            })
        );
        assert_eq!(store.dialog_link_focused_button(), Some(1));
        assert!(store.resolve_dialog_link_confirmation(false));
        assert_eq!(dialog_title(&store), Some("First"));
        assert_eq!(store.counters().show_dialog_packets, 3);
        assert_eq!(store.counters().clear_dialog_packets, 2);
    }

    #[test]
    fn waiting_screen_returns_to_suspended_confirm_link_screen() {
        let mut store = WorldStore::new();
        show_notice(&mut store, "First");
        assert!(store.open_dialog_url(
            "https://example.invalid/first".to_string(),
            "none".to_string(),
        ));
        show_notice(&mut store, "Second");

        assert!(store.wait_for_dialog_response());
        assert_eq!(store.dialog_waiting_ticks(), Some(0));
        assert_eq!(store.dialog_link_confirmation(), None);
        store.advance_dialog_waiting_ticks(DIALOG_WAITING_CLOSE_ACTIVE_TICKS);
        assert!(store.close_dialog_waiting_response());

        assert_eq!(dialog_title(&store), Some("First"));
        assert_eq!(
            store.dialog_link_confirmation(),
            Some(&DialogLinkConfirmationState {
                url: "https://example.invalid/first".to_string(),
                after_action: "none".to_string(),
            })
        );
    }

    #[test]
    fn dialog_link_options_match_vanilla_disabled_and_no_prompt_modes() {
        let mut store = WorldStore::new();
        show_notice(&mut store, "Options");
        let revision = store.dialog_screen_revision();

        store.set_dialog_link_options(false, true);
        assert!(!store.open_dialog_url(
            "https://example.invalid/disabled".to_string(),
            "close".to_string(),
        ));
        assert_eq!(store.dialog_link_confirmation(), None);
        assert!(store.take_dialog_open_url_requests().is_empty());

        store.set_dialog_link_options(true, false);
        assert!(store.open_dialog_url(
            "https://example.invalid/direct".to_string(),
            "close".to_string(),
        ));
        assert_eq!(store.dialog_screen_revision(), revision);
        assert!(store.current_dialog().is_some());
        assert_eq!(store.dialog_link_confirmation(), None);
        assert_eq!(
            store.take_dialog_open_url_requests(),
            ["https://example.invalid/direct"]
        );
        assert!(store.take_dialog_open_url_requests().is_empty());
    }

    #[test]
    fn dialog_boolean_input_values_initialize_toggle_and_clear_with_screen() {
        let entry = |name: &str, value| NbtSummaryEntry {
            name: name.to_string(),
            value,
        };
        let mut store = WorldStore::new();
        store.apply_show_dialog(ProtocolShowDialog {
            dialog: DialogHolder::Direct {
                dialog: NbtSummaryValue::Compound(vec![
                    entry(
                        "type",
                        NbtSummaryValue::String("minecraft:notice".to_string()),
                    ),
                    entry("title", NbtSummaryValue::String("Inputs".to_string())),
                    entry(
                        "inputs",
                        NbtSummaryValue::List(vec![
                            NbtSummaryValue::Compound(vec![
                                entry("key", NbtSummaryValue::String("enabled".to_string())),
                                entry("type", NbtSummaryValue::String("boolean".to_string())),
                                entry("label", NbtSummaryValue::String("Enabled".to_string())),
                                entry("initial", NbtSummaryValue::Byte(1)),
                            ]),
                            NbtSummaryValue::Compound(vec![
                                entry("key", NbtSummaryValue::String("name".to_string())),
                                entry("type", NbtSummaryValue::String("text".to_string())),
                                entry("label", NbtSummaryValue::String("Name".to_string())),
                            ]),
                        ]),
                    ),
                ]),
            },
        });

        assert_eq!(
            store.current_dialog_input_values(),
            [
                Some(DialogInputValueState::Boolean { selected: true }),
                Some(DialogInputValueState::Text {
                    value: String::new(),
                }),
            ]
        );
        assert!(store.toggle_dialog_boolean_input(0));
        assert!(!store.toggle_dialog_boolean_input(1));
        assert_eq!(
            store.current_dialog_input_values(),
            [
                Some(DialogInputValueState::Boolean { selected: false }),
                Some(DialogInputValueState::Text {
                    value: String::new(),
                }),
            ]
        );

        assert!(store.close_dialog());
        assert!(store.current_dialog_input_values().is_empty());
    }

    #[test]
    fn dialog_body_scroll_state_clamps_drags_and_resets_with_lifecycle() {
        let entry = |name: &str, value| NbtSummaryEntry {
            name: name.to_string(),
            value,
        };
        let mut store = WorldStore::new();
        assert!(store.set_dialog_body_scroll_amount(15.0, 10.0));
        assert_eq!(store.dialog_body_scroll_amount(), 10.0);
        store.begin_dialog_body_scroll_drag(40);
        assert_eq!(store.update_dialog_body_scroll_drag_y(47), Some(7));
        assert!(store.end_dialog_body_scroll_drag());
        assert!(!store.end_dialog_body_scroll_drag());
        assert!(store.set_dialog_body_scroll_amount(f64::NAN, 10.0));
        assert_eq!(store.dialog_body_scroll_amount(), 0.0);

        store.set_dialog_body_scroll_amount(8.0, 10.0);
        store.begin_dialog_body_scroll_drag(12);
        store.apply_show_dialog(ProtocolShowDialog {
            dialog: DialogHolder::Direct {
                dialog: NbtSummaryValue::Compound(vec![
                    entry(
                        "type",
                        NbtSummaryValue::String("minecraft:notice".to_string()),
                    ),
                    entry("title", NbtSummaryValue::String("Scroll".to_string())),
                ]),
            },
        });
        assert_eq!(store.dialog_body_scroll_amount(), 0.0);
        assert_eq!(store.dialog_body_scroll_drag_y(), None);

        store.set_dialog_body_scroll_amount(6.0, 10.0);
        store.begin_dialog_body_scroll_drag(20);
        assert!(store.close_dialog());
        assert_eq!(store.dialog_body_scroll_amount(), 0.0);
        assert_eq!(store.dialog_body_scroll_drag_y(), None);
    }

    #[test]
    fn dialog_item_body_focus_switches_with_inputs_and_clears_with_lifecycle() {
        let entry = |name: &str, value| NbtSummaryEntry {
            name: name.to_string(),
            value,
        };
        let mut store = WorldStore::new();
        store.apply_show_dialog(ProtocolShowDialog {
            dialog: DialogHolder::Direct {
                dialog: NbtSummaryValue::Compound(vec![
                    entry(
                        "type",
                        NbtSummaryValue::String("minecraft:notice".to_string()),
                    ),
                    entry("title", NbtSummaryValue::String("Focus".to_string())),
                    entry(
                        "body",
                        NbtSummaryValue::Compound(vec![
                            entry(
                                "type",
                                NbtSummaryValue::String("minecraft:item".to_string()),
                            ),
                            entry(
                                "item",
                                NbtSummaryValue::Compound(vec![entry(
                                    "id",
                                    NbtSummaryValue::String("minecraft:stick".to_string()),
                                )]),
                            ),
                        ]),
                    ),
                    entry(
                        "inputs",
                        NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                            entry("key", NbtSummaryValue::String("enabled".to_string())),
                            entry(
                                "type",
                                NbtSummaryValue::String("minecraft:boolean".to_string()),
                            ),
                            entry("label", NbtSummaryValue::String("Enabled".to_string())),
                        ])]),
                    ),
                ]),
            },
        });

        assert!(store.focus_dialog_body_item(0));
        assert_eq!(store.dialog_focused_body(), Some(0));
        assert_eq!(store.dialog_focused_input(), None);
        assert!(!store.focus_dialog_body_item(1));
        assert_eq!(store.dialog_focused_body(), Some(0));

        assert!(store.focus_dialog_mouse_input(0));
        assert_eq!(store.dialog_focused_body(), None);
        assert_eq!(store.dialog_focused_input(), Some(0));
        assert!(store.focus_dialog_body_item(0));
        assert_eq!(store.dialog_focused_body(), Some(0));
        assert_eq!(store.dialog_focused_input(), None);

        assert!(store.close_dialog());
        assert_eq!(store.dialog_focused_body(), None);
        assert_eq!(store.dialog_focused_input(), None);
    }

    #[test]
    fn dialog_warning_keeps_replaced_or_cleared_return_screen_hidden_until_back() {
        let entry = |name: &str, value| NbtSummaryEntry {
            name: name.to_string(),
            value,
        };
        let notice = |title: &str| ProtocolShowDialog {
            dialog: DialogHolder::Direct {
                dialog: NbtSummaryValue::Compound(vec![
                    entry(
                        "type",
                        NbtSummaryValue::String("minecraft:notice".to_string()),
                    ),
                    entry("title", NbtSummaryValue::String(title.to_string())),
                ]),
            },
        };
        let mut store = WorldStore::new();
        assert!(store.open_advancements_screen());
        store.apply_show_dialog(notice("First"));
        assert!(store.open_dialog_warning());
        assert!(store.dialog_warning_is_open());
        assert!(!store.open_dialog_warning());

        store.apply_show_dialog(notice("Second"));
        assert!(store.dialog_warning_is_open());
        let Some(DialogDataState::Notice { common, .. }) = store
            .current_dialog()
            .and_then(|dialog| dialog.data.as_ref())
        else {
            panic!("expected replacement notice");
        };
        assert_eq!(common.title.text, "Second");
        assert!(!store.advancements_screen_is_open());

        store.apply_clear_dialog();
        assert!(store.dialog_warning_is_open());
        assert!(store.current_dialog().is_none());
        assert!(store.advancements_screen_is_open());
        assert!(store.dialog_screen_is_open());
        assert!(store.close_dialog_warning());
        assert!(!store.dialog_warning_is_open());
        assert!(!store.dialog_screen_is_open());
        assert!(!store.close_dialog_warning());
    }

    #[test]
    fn dialog_number_range_value_initializes_and_drag_state_tracks_dialog_lifecycle() {
        let entry = |name: &str, value| NbtSummaryEntry {
            name: name.to_string(),
            value,
        };
        let mut store = WorldStore::new();
        store.apply_show_dialog(ProtocolShowDialog {
            dialog: DialogHolder::Direct {
                dialog: NbtSummaryValue::Compound(vec![
                    entry(
                        "type",
                        NbtSummaryValue::String("minecraft:notice".to_string()),
                    ),
                    entry("title", NbtSummaryValue::String("Inputs".to_string())),
                    entry(
                        "inputs",
                        NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                            entry("key", NbtSummaryValue::String("speed".to_string())),
                            entry("type", NbtSummaryValue::String("number_range".to_string())),
                            entry("label", NbtSummaryValue::String("Speed".to_string())),
                            entry("start", NbtSummaryValue::Float(10.0f32.to_bits())),
                            entry("end", NbtSummaryValue::Float(0.0f32.to_bits())),
                            entry("initial", NbtSummaryValue::Float(7.5f32.to_bits())),
                        ])]),
                    ),
                ]),
            },
        });

        assert_eq!(
            store.current_dialog_input_values(),
            [Some(DialogInputValueState::NumberRange {
                slider_value_bits: 0.25f64.to_bits(),
            })]
        );
        assert!(store.begin_dialog_number_range_drag(0));
        assert_eq!(store.dialog_dragging_number_range_input(), Some(0));
        assert!(store.focus_dialog_mouse_input(0));
        assert_eq!(store.dialog_focused_input(), Some(0));
        assert!(store.dialog_number_range_keyboard_can_change(0));
        assert!(store.toggle_dialog_number_range_keyboard_change(0));
        assert!(!store.dialog_number_range_keyboard_can_change(0));
        assert!(store.set_dialog_number_range_slider_value(0, 2.0));
        assert_eq!(
            store.current_dialog_input_values(),
            [Some(DialogInputValueState::NumberRange {
                slider_value_bits: 1.0f64.to_bits(),
            })]
        );

        assert!(store.close_dialog());
        assert_eq!(store.dialog_dragging_number_range_input(), None);
        assert_eq!(store.dialog_focused_input(), None);
        assert!(store.current_dialog_input_values().is_empty());
    }

    #[test]
    fn dialog_single_option_uses_marked_initial_and_updates_selected_index() {
        let entry = |name: &str, value| NbtSummaryEntry {
            name: name.to_string(),
            value,
        };
        let mut store = WorldStore::new();
        store.apply_show_dialog(ProtocolShowDialog {
            dialog: DialogHolder::Direct {
                dialog: NbtSummaryValue::Compound(vec![
                    entry(
                        "type",
                        NbtSummaryValue::String("minecraft:notice".to_string()),
                    ),
                    entry("title", NbtSummaryValue::String("Inputs".to_string())),
                    entry(
                        "inputs",
                        NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                            entry("key", NbtSummaryValue::String("mode".to_string())),
                            entry("type", NbtSummaryValue::String("single_option".to_string())),
                            entry("label", NbtSummaryValue::String("Mode".to_string())),
                            entry(
                                "options",
                                NbtSummaryValue::List(vec![
                                    NbtSummaryValue::String("first".to_string()),
                                    NbtSummaryValue::Compound(vec![
                                        entry("id", NbtSummaryValue::String("second".to_string())),
                                        entry("initial", NbtSummaryValue::Byte(1)),
                                    ]),
                                ]),
                            ),
                        ])]),
                    ),
                ]),
            },
        });

        assert_eq!(
            store.current_dialog_input_values(),
            [Some(DialogInputValueState::SingleOption {
                selected_index: 1,
            })]
        );
        assert!(store.set_dialog_single_option_index(0, 0));
        assert_eq!(
            store.current_dialog_input_values(),
            [Some(DialogInputValueState::SingleOption {
                selected_index: 0,
            })]
        );
    }

    #[test]
    fn dialog_single_line_text_tracks_utf16_bounded_value_selection_and_drag() {
        let entry = |name: &str, value| NbtSummaryEntry {
            name: name.to_string(),
            value,
        };
        let mut store = WorldStore::new();
        store.apply_show_dialog(ProtocolShowDialog {
            dialog: DialogHolder::Direct {
                dialog: NbtSummaryValue::Compound(vec![
                    entry(
                        "type",
                        NbtSummaryValue::String("minecraft:notice".to_string()),
                    ),
                    entry("title", NbtSummaryValue::String("Inputs".to_string())),
                    entry(
                        "inputs",
                        NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                            entry("key", NbtSummaryValue::String("name".to_string())),
                            entry("type", NbtSummaryValue::String("text".to_string())),
                            entry("label", NbtSummaryValue::String("Name".to_string())),
                            entry("initial", NbtSummaryValue::String("A😀".to_string())),
                            entry("max_length", NbtSummaryValue::Int(4)),
                        ])]),
                    ),
                ]),
            },
        });

        assert_eq!(
            store.current_dialog_input_values(),
            [Some(DialogInputValueState::Text {
                value: "A😀".to_string(),
            })]
        );
        assert!(store.focus_dialog_text_input(0, 1, false));
        assert_eq!(store.dialog_focused_input(), Some(0));
        assert!(store.focus_dialog_text_input(0, 2, true));
        assert_eq!(
            store.dialog_text_edit_state(),
            Some(DialogTextEditState {
                input_index: 0,
                cursor: 2,
                selection: 1,
                scroll_amount: 0,
            })
        );
        assert!(store.begin_dialog_text_input_drag(0));
        assert_eq!(store.dialog_dragging_text_input(), Some(0));
        assert!(!store.set_dialog_text_input_value_and_selection(0, "A😀BC".to_string(), 4, 4,));
        assert!(store.set_dialog_text_input_value_and_selection(0, "A😀B".to_string(), 3, 3,));
        assert_eq!(store.end_dialog_text_input_drag(), Some(0));

        assert!(store.close_dialog());
        assert_eq!(store.dialog_text_edit_state(), None);
        assert_eq!(store.dialog_dragging_text_input(), None);
        assert!(store.current_dialog_input_values().is_empty());
    }

    #[test]
    fn dialog_clear_restores_world_owned_previous_screen() {
        let previous_book = BookScreenState::View(crate::BookViewScreenState {
            generation: 1,
            hand: InteractionHand::MainHand,
            pages: vec![
                crate::BookViewPageState {
                    raw: bbb_protocol::RichTextComponent::plain("First"),
                    filtered: None,
                },
                crate::BookViewPageState {
                    raw: bbb_protocol::RichTextComponent::plain("Second"),
                    filtered: None,
                },
            ],
            current_page: 1,
        });
        let mut book_store = WorldStore::new();
        book_store.client_ui.current_book = Some(previous_book.clone());
        book_store.apply_show_dialog(ProtocolShowDialog {
            dialog: DialogHolder::Reference { registry_id: 1 },
        });
        assert_eq!(book_store.current_book(), None);

        // Replacing one dialog with another keeps the original non-dialog previous screen.
        book_store.apply_show_dialog(ProtocolShowDialog {
            dialog: DialogHolder::Reference { registry_id: 2 },
        });
        book_store.apply_clear_dialog();
        assert_eq!(book_store.current_book(), Some(&previous_book));

        let mut advancements_store = WorldStore::new();
        assert!(advancements_store.open_advancements_screen());
        advancements_store.apply_show_dialog(ProtocolShowDialog {
            dialog: DialogHolder::Reference { registry_id: 3 },
        });
        assert!(!advancements_store.advancements_screen_is_open());
        advancements_store.apply_clear_dialog();
        assert!(advancements_store.advancements_screen_is_open());

        let mut stats_store = WorldStore::new();
        assert!(stats_store.open_stats_screen());
        stats_store.apply_show_dialog(ProtocolShowDialog {
            dialog: DialogHolder::Reference { registry_id: 4 },
        });
        assert!(!stats_store.stats_screen_is_open());
        stats_store.apply_clear_dialog();
        assert!(stats_store.stats_screen_is_open());
        assert!(stats_store.stats_screen_is_loading());
    }

    #[test]
    fn local_dialog_close_restores_previous_screen_without_counting_a_packet() {
        let mut store = WorldStore::new();
        assert!(store.open_advancements_screen());
        store.apply_show_dialog(ProtocolShowDialog {
            dialog: DialogHolder::Reference { registry_id: 3 },
        });

        assert!(store.close_dialog());
        assert!(store.advancements_screen_is_open());
        assert_eq!(store.counters().clear_dialog_packets, 0);
        assert!(!store.close_dialog());
        assert_eq!(store.counters().clear_dialog_packets, 0);
    }

    #[test]
    fn waiting_dialog_preserves_previous_screen_and_ignores_clear_packet() {
        let mut store = WorldStore::new();
        assert!(store.open_advancements_screen());
        store.apply_show_dialog(ProtocolShowDialog {
            dialog: DialogHolder::Reference { registry_id: 3 },
        });

        assert!(store.wait_for_dialog_response());
        assert_eq!(store.current_dialog(), None);
        assert_eq!(store.dialog_waiting_ticks(), Some(0));
        assert!(!store.advancements_screen_is_open());
        store.apply_clear_dialog();
        assert_eq!(store.dialog_waiting_ticks(), Some(0));
        assert_eq!(store.counters().clear_dialog_packets, 1);

        store.advance_dialog_waiting_ticks(100);
        assert!(!store.dialog_waiting_can_close());
        assert!(!store.close_dialog_waiting_response());
        store.advance_dialog_waiting_ticks(1);
        assert!(store.dialog_waiting_can_close());
        assert!(store.close_dialog_waiting_response());
        assert!(store.advancements_screen_is_open());
    }

    #[test]
    fn show_dialog_replaces_waiting_screen_without_losing_previous_screen() {
        let mut store = WorldStore::new();
        assert!(store.open_advancements_screen());
        store.apply_show_dialog(ProtocolShowDialog {
            dialog: DialogHolder::Reference { registry_id: 3 },
        });
        assert!(store.wait_for_dialog_response());

        store.apply_show_dialog(ProtocolShowDialog {
            dialog: DialogHolder::Reference { registry_id: 4 },
        });

        assert!(store.current_dialog().is_some());
        assert_eq!(store.dialog_waiting_ticks(), None);
        assert!(store.close_dialog());
        assert!(store.advancements_screen_is_open());
    }

    #[test]
    fn clear_dialog_only_restores_while_dialog_is_active() {
        let mut store = WorldStore::new();
        assert!(store.open_advancements_screen());
        store.apply_clear_dialog();
        assert!(store.advancements_screen_is_open());

        store.apply_show_dialog(ProtocolShowDialog {
            dialog: DialogHolder::Reference { registry_id: 1 },
        });
        assert!(store.open_stats_screen());
        assert_eq!(store.current_dialog(), None);

        // The later screen replaced the dialog, so a late clear packet must not revive its
        // now-stale previous screen.
        store.apply_clear_dialog();
        assert!(store.stats_screen_is_open());
        assert!(!store.advancements_screen_is_open());

        let mut container_store = WorldStore::new();
        assert!(container_store.open_advancements_screen());
        container_store.apply_show_dialog(ProtocolShowDialog {
            dialog: DialogHolder::Reference { registry_id: 2 },
        });
        container_store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X1_ID,
            title: bbb_protocol::RichTextComponent::plain("Container"),
        });
        assert_eq!(container_store.current_dialog(), None);
        container_store.apply_clear_dialog();
        assert_eq!(container_store.open_container_id(), Some(7));
        assert!(!container_store.advancements_screen_is_open());
    }

    #[test]
    fn referenced_dialogs_use_session_holder_order_and_retain_resolution_errors() {
        let mut store = WorldStore::new();
        store.record_registry_entries(
            DIALOG_REGISTRY,
            0,
            vec![
                crate::RegistryPacketEntry::stub("example:stub"),
                crate::RegistryPacketEntry::with_raw_data(
                    "example:custom",
                    dialog_nbt("minecraft:notice"),
                ),
                crate::RegistryPacketEntry::with_raw_data("example:malformed", vec![10, 0, 0]),
            ],
        );

        store.apply_show_dialog(ProtocolShowDialog {
            dialog: DialogHolder::Reference { registry_id: 1 },
        });
        let resolved = store.current_dialog().unwrap();
        assert_eq!(resolved.resource_id.as_deref(), Some("example:custom"));
        assert!(matches!(
            resolved.dialog,
            Some(NbtSummaryValue::Compound(_))
        ));
        assert_eq!(resolved.resolution_error, None);
        assert!(matches!(
            resolved.data,
            Some(crate::DialogDataState::Notice { .. })
        ));
        assert_eq!(resolved.semantic_error, None);

        store.apply_show_dialog(ProtocolShowDialog {
            dialog: DialogHolder::Reference { registry_id: 0 },
        });
        let stub = store.current_dialog().unwrap();
        assert_eq!(stub.resource_id.as_deref(), Some("example:stub"));
        assert_eq!(stub.dialog, None);
        assert!(stub
            .resolution_error
            .as_deref()
            .unwrap()
            .contains("has no synchronized payload"));

        store.apply_show_dialog(ProtocolShowDialog {
            dialog: DialogHolder::Reference { registry_id: 2 },
        });
        let malformed = store.current_dialog().unwrap();
        assert_eq!(malformed.resource_id.as_deref(), Some("example:malformed"));
        assert_eq!(malformed.dialog, None);
        assert!(malformed
            .resolution_error
            .as_deref()
            .unwrap()
            .contains("trailing bytes after network nbt root"));

        store.apply_show_dialog(ProtocolShowDialog {
            dialog: DialogHolder::Reference { registry_id: 99 },
        });
        let missing = store.current_dialog().unwrap();
        assert_eq!(missing.resource_id, None);
        assert_eq!(missing.dialog, None);
        assert_eq!(
            missing.resolution_error.as_deref(),
            Some("missing minecraft:dialog holder 99")
        );
    }

    fn dialog_nbt(dialog_type: &str) -> Vec<u8> {
        let mut bytes = vec![10, 8, 0, 4, b't', b'y', b'p', b'e'];
        bytes.extend_from_slice(&(dialog_type.len() as u16).to_be_bytes());
        bytes.extend_from_slice(dialog_type.as_bytes());
        bytes.extend_from_slice(&[8, 0, 5, b't', b'i', b't', b'l', b'e', 0, 5]);
        bytes.extend_from_slice(b"Title");
        bytes.push(0);
        bytes
    }

    fn pause_notice_registry_entry(resource_id: &str, title: &str) -> crate::RegistryPacketEntry {
        crate::RegistryPacketEntry::with_known_pack_data_for_test(
            resource_id,
            format!(r#"{{"type":"minecraft:notice","title":"{title}"}}"#).into_bytes(),
        )
    }

    fn apply_pause_screen_dialog_tag(store: &mut WorldStore, entries: Vec<i32>) {
        store.apply_update_tags(UpdateTags {
            registries: vec![RegistryTags {
                registry: DIALOG_REGISTRY.to_string(),
                tags: vec![TagNetworkPayload {
                    tag: PAUSE_SCREEN_ADDITIONS_TAG.to_string(),
                    entries,
                }],
            }],
        });
    }

    fn show_notice(store: &mut WorldStore, title: &str) {
        store.apply_show_dialog(ProtocolShowDialog {
            dialog: DialogHolder::Direct {
                dialog: NbtSummaryValue::Compound(vec![
                    NbtSummaryEntry {
                        name: "type".to_string(),
                        value: NbtSummaryValue::String("minecraft:notice".to_string()),
                    },
                    NbtSummaryEntry {
                        name: "title".to_string(),
                        value: NbtSummaryValue::String(title.to_string()),
                    },
                ]),
            },
        });
    }

    fn dialog_title(store: &WorldStore) -> Option<&str> {
        match store.current_dialog()?.data.as_ref()? {
            DialogDataState::Notice { common, .. } => Some(&common.title.text),
            _ => None,
        }
    }

    #[test]
    fn tracks_code_of_conduct_text() {
        let mut store = WorldStore::new();
        let text = "Keep the server friendly.";

        store.apply_code_of_conduct(text.to_string());

        assert_eq!(
            store.last_code_of_conduct(),
            Some(&CodeOfConductState {
                text: text.to_string(),
                text_hash: code_of_conduct_text_hash(text),
            })
        );
        assert_eq!(store.counters().code_of_conduct_packets, 1);
        assert_eq!(store.counters().last_code_of_conduct_len, text.len());
    }

    #[test]
    fn code_of_conduct_hash_matches_java_string_hash_code() {
        assert_eq!(code_of_conduct_text_hash(""), 0);
        assert_eq!(code_of_conduct_text_hash("abc"), 96354);
        assert_eq!(code_of_conduct_text_hash("Aa"), 2112);
        assert_eq!(code_of_conduct_text_hash("BB"), 2112);
        assert_eq!(code_of_conduct_text_hash("😀"), 1_772_899);
    }

    #[test]
    fn tracks_client_ui_open_requests() {
        let mut store = WorldStore::new();
        store.apply_add_entity(test_entity(42, VANILLA_ENTITY_TYPE_HORSE_ID));
        let sign_pos = BlockPos {
            x: -5,
            y: 70,
            z: 12,
        };
        insert_test_sign(&mut store, sign_pos);

        store.apply_mount_screen_open(ProtocolMountScreenOpen {
            container_id: 11,
            inventory_columns: 5,
            entity_id: 42,
        });
        store.apply_open_book(ProtocolOpenBook {
            hand: InteractionHand::OffHand,
        });
        store.apply_open_sign_editor(ProtocolOpenSignEditor {
            pos: ProtocolBlockPos {
                x: -5,
                y: 70,
                z: 12,
            },
            is_front_text: false,
        });

        assert_eq!(
            store.last_mount_screen(),
            Some(&MountScreenState {
                container_id: 11,
                inventory_columns: 5,
                entity_id: 42,
            })
        );
        let open_container = store.inventory().open_container.as_ref().unwrap();
        assert_eq!(open_container.container_id, 11);
        assert_eq!(open_container.menu_type_id, None);
        assert_eq!(
            open_container.mount,
            Some(MountScreenState {
                container_id: 11,
                inventory_columns: 5,
                entity_id: 42,
            })
        );
        assert_eq!(
            store.last_open_book(),
            Some(&OpenBookState {
                hand: "off_hand".to_string(),
            })
        );
        assert_eq!(
            store.last_open_sign_editor(),
            Some(&OpenSignEditorState {
                pos: BlockPos {
                    x: -5,
                    y: 70,
                    z: 12,
                },
                is_front_text: false,
                lines: [
                    "raw back".to_string(),
                    String::new(),
                    String::new(),
                    String::new(),
                ],
                filtered_lines: Some([
                    "filtered back".to_string(),
                    String::new(),
                    String::new(),
                    String::new(),
                ]),
            })
        );
        let editor = store.last_open_sign_editor().unwrap();
        assert_eq!(editor.lines_for_filter(false)[0], "raw back");
        assert_eq!(editor.lines_for_filter(true)[0], "filtered back");

        let counters = store.counters();
        assert_eq!(counters.mount_screen_open_packets, 1);
        assert_eq!(counters.open_book_packets, 1);
        assert_eq!(counters.open_sign_editor_packets, 1);
        assert_eq!(counters.open_sign_editor_updates_applied, 1);
        assert_eq!(counters.open_sign_editor_updates_ignored, 0);
    }

    #[test]
    fn open_sign_editor_ignores_missing_sign_without_replacing_screen() {
        let mut store = WorldStore::new();
        store.apply_show_dialog(ProtocolShowDialog {
            dialog: DialogHolder::Reference { registry_id: 7 },
        });
        let packet = ProtocolOpenSignEditor {
            pos: ProtocolBlockPos { x: 3, y: 64, z: -9 },
            is_front_text: true,
        };

        store.apply_open_sign_editor(packet);
        assert_eq!(store.last_open_sign_editor(), None);
        assert!(store.current_dialog().is_some());
        assert_eq!(store.counters().open_sign_editor_packets, 1);
        assert_eq!(store.counters().open_sign_editor_updates_applied, 0);
        assert_eq!(store.counters().open_sign_editor_updates_ignored, 1);

        let pos = BlockPos { x: 3, y: 64, z: -9 };
        insert_test_sign(&mut store, pos);
        store.apply_open_sign_editor(packet);
        assert_eq!(store.last_open_sign_editor().unwrap().pos, pos);
        assert_eq!(store.current_dialog(), None);
        assert_eq!(store.counters().open_sign_editor_packets, 2);
        assert_eq!(store.counters().open_sign_editor_updates_applied, 1);
        assert_eq!(store.counters().open_sign_editor_updates_ignored, 1);
    }

    #[test]
    fn mount_screen_open_ignores_missing_and_non_mount_entities() {
        let mut store = WorldStore::new();
        store.apply_show_dialog(ProtocolShowDialog {
            dialog: DialogHolder::Direct {
                dialog: NbtSummaryValue::Compound(Vec::new()),
            },
        });

        store.apply_mount_screen_open(ProtocolMountScreenOpen {
            container_id: 11,
            inventory_columns: 5,
            entity_id: 42,
        });
        assert_eq!(store.last_mount_screen(), None);
        assert_eq!(store.inventory().open_container, None);

        store.apply_add_entity(test_entity(42, VANILLA_ENTITY_TYPE_PLAYER_ID));
        store.apply_mount_screen_open(ProtocolMountScreenOpen {
            container_id: 11,
            inventory_columns: 5,
            entity_id: 42,
        });
        assert_eq!(store.last_mount_screen(), None);
        assert_eq!(store.inventory().open_container, None);
        assert!(store.current_dialog().is_some());

        store.apply_add_entity(test_entity(42, VANILLA_ENTITY_TYPE_HORSE_ID));
        store.apply_mount_screen_open(ProtocolMountScreenOpen {
            container_id: 11,
            inventory_columns: 5,
            entity_id: 42,
        });
        assert!(store.last_mount_screen().is_some());
        assert!(store.inventory().open_container.is_some());
        assert_eq!(store.current_dialog(), None);
        assert_eq!(store.counters().mount_screen_open_packets, 3);
    }

    #[test]
    fn open_book_without_book_access_records_request_without_screen() {
        let mut store = WorldStore::new();
        store.apply_show_dialog(ProtocolShowDialog {
            dialog: DialogHolder::Reference { registry_id: 7 },
        });
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(42, 1),
        });

        store.apply_open_book(ProtocolOpenBook {
            hand: InteractionHand::MainHand,
        });

        assert_eq!(
            store.last_open_book(),
            Some(&OpenBookState {
                hand: "main_hand".to_string(),
            })
        );
        assert_eq!(store.current_book(), None);
        assert!(store.current_dialog().is_some());
        assert_eq!(store.counters().open_book_packets, 1);
    }

    #[test]
    fn open_book_from_held_written_book_tracks_active_screen() {
        let mut store = WorldStore::new();
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: written_book_stack(vec!["First", "Second"]),
        });

        store.apply_open_book(ProtocolOpenBook {
            hand: InteractionHand::MainHand,
        });

        let book = store.current_book().unwrap();
        assert!(book.is_view());
        assert_eq!(book.generation(), 1);
        assert_eq!(book.hand(), InteractionHand::MainHand);
        assert_eq!(
            book.plain_pages_for_filter(false),
            vec!["First".to_string(), "Second".to_string()]
        );
        assert_eq!(book.current_page(), 0);
        assert!(store.turn_current_book_page(1));
        assert_eq!(store.current_book().unwrap().current_page(), 1);
        assert!(!store.turn_current_book_page(1));
        assert_eq!(store.current_book().unwrap().current_page(), 1);
        assert!(store.close_current_book());
        assert_eq!(store.current_book(), None);
    }

    #[test]
    fn open_book_from_held_writable_book_uses_raw_pages() {
        let mut store = WorldStore::new();
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 40,
            item: writable_book_stack(vec!["Draft page"]),
        });

        store.apply_open_book(ProtocolOpenBook {
            hand: InteractionHand::OffHand,
        });

        let book = store.current_book().unwrap();
        assert!(book.is_view());
        assert_eq!(book.hand(), InteractionHand::OffHand);
        assert_eq!(
            book.plain_pages_for_filter(false),
            vec!["Draft page".to_string()]
        );
        assert_eq!(book.current_page(), 0);
    }

    #[test]
    fn advancements_screen_open_close_tracks_local_screen() {
        let mut store = WorldStore::new();

        assert!(!store.advancements_screen_is_open());
        assert!(store.open_advancements_screen());
        assert!(store.advancements_screen_is_open());
        assert!(!store.open_advancements_screen());
        assert!(store.close_advancements_screen());
        assert!(!store.advancements_screen_is_open());
        assert!(!store.close_advancements_screen());
    }

    #[test]
    fn stats_screen_open_close_tracks_local_screen() {
        let mut store = WorldStore::new();

        assert!(!store.stats_screen_is_open());
        assert!(store.open_stats_screen());
        assert!(store.stats_screen_is_open());
        assert!(!store.open_stats_screen());
        assert!(store.close_stats_screen());
        assert!(!store.stats_screen_is_open());
        assert!(!store.close_stats_screen());
    }

    #[test]
    fn other_client_screens_replace_advancements_screen() {
        let mut store = WorldStore::new();
        assert!(store.open_advancements_screen());

        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: written_book_stack(vec!["Page"]),
        });
        store.apply_open_book(ProtocolOpenBook {
            hand: InteractionHand::MainHand,
        });

        assert!(!store.advancements_screen_is_open());
        assert!(store.current_book().is_some());

        assert!(store.open_advancements_screen());
        assert!(store.current_book().is_none());
        store.open_local_inventory();
        assert!(!store.advancements_screen_is_open());
        assert!(store.local_inventory_is_open());

        assert!(store.open_stats_screen());
        assert!(store.stats_screen_is_open());
        assert!(store.open_advancements_screen());
        assert!(!store.stats_screen_is_open());
        assert!(store.advancements_screen_is_open());
    }

    #[test]
    fn tracks_client_ui_pong_response() {
        let mut store = WorldStore::new();

        store.apply_pong_response(ProtocolPongResponse { time: 123456789 });

        assert_eq!(
            store.last_pong_response(),
            Some(&PongResponseState { time: 123456789 })
        );
        assert_eq!(store.counters().pong_response_packets, 1);
    }

    fn item_stack(item_id: i32, count: i32) -> ItemStackSummary {
        ItemStackSummary {
            item_id: Some(item_id),
            count,
            component_patch: DataComponentPatchSummary::default(),
        }
    }

    fn written_book_stack(pages: Vec<&str>) -> ItemStackSummary {
        let mut stack = item_stack(42, 1);
        let pages: Vec<String> = pages.into_iter().map(str::to_string).collect();
        let page_filters = vec![None; pages.len()];
        stack.component_patch.written_book = Some(WrittenBookContentSummary {
            title: "Guide".to_string(),
            title_filter: None,
            author: "Alex".to_string(),
            generation: 0,
            pages,
            page_filters,
            resolved: true,
        });
        stack
    }

    fn writable_book_stack(pages: Vec<&str>) -> ItemStackSummary {
        let mut stack = item_stack(43, 1);
        stack
            .component_patch
            .added_type_ids
            .push(VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID);
        stack.component_patch.writable_book_pages = pages.into_iter().map(str::to_string).collect();
        stack
    }

    fn test_entity(id: i32, entity_type_id: i32) -> ProtocolAddEntity {
        ProtocolAddEntity {
            id,
            uuid: Uuid::from_u128(id as u128),
            entity_type_id,
            position: ProtocolVec3d {
                x: 0.0,
                y: 64.0,
                z: 0.0,
            },
            delta_movement: ProtocolVec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        }
    }

    fn insert_test_sign(store: &mut WorldStore, pos: BlockPos) {
        store.insert_decoded_chunk(crate::ChunkColumn {
            pos: crate::ChunkPos {
                x: pos.x.div_euclid(16),
                z: pos.z.div_euclid(16),
            },
            state: crate::ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: Vec::new(),
            block_entities: vec![crate::BlockEntityRecord {
                local_x: pos.x.rem_euclid(16) as u8,
                y: i16::try_from(pos.y).unwrap(),
                local_z: pos.z.rem_euclid(16) as u8,
                type_id: VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID,
                raw_nbt: vec![0],
                nbt: None,
                sign_text: Some(crate::SignBlockEntityTextState {
                    front: test_sign_side("raw front", "filtered front"),
                    back: test_sign_side("raw back", "filtered back"),
                    is_waxed: false,
                }),
                vault_shared_data: None,
                decorated_pot_sherds: None,
                banner_patterns: None,
                end_gateway: None,
                spawner: None,
                command_block: None,
                comparator_output: None,
                structure_block: None,
                jigsaw_block: None,
                test_block: None,
                test_instance_block: None,
                note_block_sound: None,
                skull_profile: None,
            }],
            light: crate::LightData::default(),
        });
    }

    fn test_sign_side(raw: &str, filtered: &str) -> crate::SignTextSideState {
        let line = |text: &str| {
            vec![bbb_protocol::StyledTextRun {
                text: text.to_string(),
                style: bbb_protocol::ComponentStyle::default(),
            }]
        };
        crate::SignTextSideState {
            lines: [line(raw), Vec::new(), Vec::new(), Vec::new()],
            filtered_lines: Some([line(filtered), Vec::new(), Vec::new(), Vec::new()]),
            color: crate::SignTextDyeColor::Black,
            has_glowing_text: false,
        }
    }

    #[test]
    fn tracks_client_ui_ghost_recipe_request() {
        let mut store = WorldStore::new();
        assert!(store.open_local_inventory());
        let recipe_display = stonecutter_recipe_display(100, 101, 102);

        store.apply_place_ghost_recipe(ProtocolPlaceGhostRecipe {
            container_id: 0,
            recipe_display: recipe_display.clone(),
        });

        assert_eq!(
            store.last_ghost_recipe(),
            Some(&GhostRecipeState {
                container_id: 0,
                recipe_display_type_id: VANILLA_RECIPE_DISPLAY_TYPE_STONECUTTER_ID,
                recipe_display_type: "stonecutter".to_string(),
                recipe_display_body_len: 7,
                recipe_display: Some(recipe_display),
                slots: vec![GhostRecipeSlotState {
                    slot_id: 0,
                    items: vec![item_stack(101, 1)],
                    is_result: true,
                }],
            })
        );
        assert_eq!(store.counters().ghost_recipe_packets, 1);
        assert_eq!(store.counters().ghost_recipe_updates_applied, 1);
        assert_eq!(store.counters().ghost_recipe_updates_ignored, 0);
    }

    #[test]
    fn ghost_recipe_ignores_inactive_container_without_caching_for_later() {
        let mut store = WorldStore::new();
        let packet = ProtocolPlaceGhostRecipe {
            container_id: 9,
            recipe_display: stonecutter_recipe_display(100, 101, 102),
        };

        store.apply_place_ghost_recipe(packet.clone());
        assert_eq!(store.last_ghost_recipe(), None);
        assert_eq!(store.counters().ghost_recipe_updates_ignored, 1);

        assert!(store.open_local_inventory());
        store.apply_place_ghost_recipe(packet);
        assert_eq!(store.last_ghost_recipe(), None);
        assert_eq!(store.counters().ghost_recipe_packets, 2);
        assert_eq!(store.counters().ghost_recipe_updates_applied, 0);
        assert_eq!(store.counters().ghost_recipe_updates_ignored, 2);
    }

    #[test]
    fn clears_client_ui_ghost_recipe_without_packet_counter() {
        let mut store = WorldStore::new();
        assert!(store.open_local_inventory());

        store.apply_place_ghost_recipe(ProtocolPlaceGhostRecipe {
            container_id: 0,
            recipe_display: stonecutter_recipe_display(100, 101, 102),
        });

        assert!(store.clear_ghost_recipe());
        assert_eq!(store.last_ghost_recipe(), None);
        assert!(!store.clear_ghost_recipe());
        assert_eq!(store.counters().ghost_recipe_packets, 1);
    }

    #[test]
    fn ghost_recipe_places_inputs_in_the_active_crafting_grid() {
        let mut store = WorldStore::new();
        assert!(store.open_local_inventory());
        let shaped = crafting_shaped_recipe_display(1, 1, vec![110], 120);
        store.apply_place_ghost_recipe(ProtocolPlaceGhostRecipe {
            container_id: 0,
            recipe_display: shaped.clone(),
        });
        assert_eq!(
            ghost_slot_ids(store.last_ghost_recipe().unwrap()),
            vec![(0, true), (1, false)]
        );

        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
            title: bbb_protocol::RichTextComponent::plain("Crafting"),
        });
        assert_eq!(store.last_ghost_recipe(), None);
        store.apply_place_ghost_recipe(ProtocolPlaceGhostRecipe {
            container_id: 7,
            recipe_display: shaped,
        });
        assert_eq!(
            ghost_slot_ids(store.last_ghost_recipe().unwrap()),
            vec![(0, true), (5, false)]
        );

        assert!(store.close_local_container(7));
        assert!(store.open_local_inventory());
        let shapeless = crafting_shapeless_recipe_display(vec![101, 102, 103, 104, 105], 121);
        store.apply_place_ghost_recipe(ProtocolPlaceGhostRecipe {
            container_id: 0,
            recipe_display: shapeless,
        });
        assert_eq!(
            ghost_slot_ids(store.last_ghost_recipe().unwrap()),
            vec![(0, true), (1, false), (2, false), (3, false), (4, false)]
        );
    }

    #[test]
    fn furnace_family_ghost_recipe_snapshots_fuel_slot_emptiness() {
        for menu_type_id in [
            VANILLA_MENU_TYPE_FURNACE_ID,
            VANILLA_MENU_TYPE_BLAST_FURNACE_ID,
            VANILLA_MENU_TYPE_SMOKER_ID,
        ] {
            let mut store = WorldStore::new();
            store.apply_open_screen(ProtocolOpenScreen {
                container_id: 7,
                menu_type_id,
                title: bbb_protocol::RichTextComponent::plain("Furnace"),
            });
            let display = furnace_recipe_display(110, 111, 112);
            store.apply_place_ghost_recipe(ProtocolPlaceGhostRecipe {
                container_id: 7,
                recipe_display: display.clone(),
            });
            assert_eq!(
                ghost_slot_ids(store.last_ghost_recipe().unwrap()),
                vec![(2, true), (0, false), (1, false)]
            );

            assert!(store.apply_container_set_slot(ProtocolContainerSetSlot {
                container_id: 7,
                state_id: 1,
                slot: 1,
                item: item_stack(113, 1),
            }));
            store.apply_place_ghost_recipe(ProtocolPlaceGhostRecipe {
                container_id: 7,
                recipe_display: display,
            });
            assert_eq!(
                ghost_slot_ids(store.last_ghost_recipe().unwrap()),
                vec![(2, true), (0, false)]
            );

            store.apply_place_ghost_recipe(ProtocolPlaceGhostRecipe {
                container_id: 7,
                recipe_display: stonecutter_recipe_display(114, 115, 116),
            });
            assert_eq!(
                ghost_slot_ids(store.last_ghost_recipe().unwrap()),
                vec![(2, true)]
            );
        }
    }

    #[test]
    fn replacing_the_screen_clears_its_ghost_recipe_snapshot() {
        let mut store = WorldStore::new();
        assert!(store.open_local_inventory());
        store.apply_place_ghost_recipe(ProtocolPlaceGhostRecipe {
            container_id: 0,
            recipe_display: stonecutter_recipe_display(100, 101, 102),
        });
        assert!(store.last_ghost_recipe().is_some());

        assert!(store.open_stats_screen());
        assert_eq!(store.last_ghost_recipe(), None);
    }

    fn ghost_slot_ids(state: &GhostRecipeState) -> Vec<(i16, bool)> {
        state
            .slots
            .iter()
            .map(|slot| (slot.slot_id, slot.is_result))
            .collect()
    }

    fn crafting_shapeless_recipe_display(
        ingredients: Vec<i32>,
        result: i32,
    ) -> bbb_protocol::packets::RecipeDisplaySummary {
        let ingredients = ingredients
            .into_iter()
            .map(item_slot_display)
            .collect::<Vec<_>>();
        bbb_protocol::packets::RecipeDisplaySummary {
            display_type: bbb_protocol::packets::RecipeDisplayType::CraftingShapeless,
            raw_body: Vec::new(),
            crafting: Some(Box::new(
                bbb_protocol::packets::CraftingRecipeDisplaySummary::Shapeless {
                    ingredients,
                    result: item_slot_display(result),
                    crafting_station: empty_slot_display(),
                },
            )),
            furnace: None,
            stonecutter: None,
            smithing: None,
        }
    }

    fn crafting_shaped_recipe_display(
        width: i32,
        height: i32,
        ingredients: Vec<i32>,
        result: i32,
    ) -> bbb_protocol::packets::RecipeDisplaySummary {
        let ingredients = ingredients
            .into_iter()
            .map(item_slot_display)
            .collect::<Vec<_>>();
        bbb_protocol::packets::RecipeDisplaySummary {
            display_type: bbb_protocol::packets::RecipeDisplayType::CraftingShaped,
            raw_body: Vec::new(),
            crafting: Some(Box::new(
                bbb_protocol::packets::CraftingRecipeDisplaySummary::Shaped {
                    width,
                    height,
                    ingredients,
                    result: item_slot_display(result),
                    crafting_station: empty_slot_display(),
                },
            )),
            furnace: None,
            stonecutter: None,
            smithing: None,
        }
    }

    fn furnace_recipe_display(
        ingredient: i32,
        fuel: i32,
        result: i32,
    ) -> bbb_protocol::packets::RecipeDisplaySummary {
        bbb_protocol::packets::RecipeDisplaySummary {
            display_type: bbb_protocol::packets::RecipeDisplayType::Furnace,
            raw_body: Vec::new(),
            crafting: None,
            furnace: Some(Box::new(
                bbb_protocol::packets::FurnaceRecipeDisplaySummary {
                    ingredient: item_slot_display(ingredient),
                    fuel: item_slot_display(fuel),
                    result: item_slot_display(result),
                    crafting_station: empty_slot_display(),
                    duration: 200,
                    experience_bits: 0.35_f32.to_bits(),
                },
            )),
            stonecutter: None,
            smithing: None,
        }
    }

    fn stonecutter_recipe_display(
        input: i32,
        result: i32,
        crafting_station: i32,
    ) -> bbb_protocol::packets::RecipeDisplaySummary {
        let input = item_slot_display(input);
        let result = item_slot_display(result);
        let crafting_station = item_slot_display(crafting_station);
        let mut raw_body = Encoder::new();
        raw_body.write_var_i32(bbb_protocol::packets::RecipeDisplayType::Stonecutter.id());
        raw_body.write_bytes(&input.raw_payload);
        raw_body.write_bytes(&result.raw_payload);
        raw_body.write_bytes(&crafting_station.raw_payload);
        bbb_protocol::packets::RecipeDisplaySummary {
            display_type: bbb_protocol::packets::RecipeDisplayType::Stonecutter,
            raw_body: raw_body.into_inner(),
            crafting: None,
            furnace: None,
            stonecutter: Some(Box::new(
                bbb_protocol::packets::StonecutterRecipeDisplaySummary {
                    input,
                    result,
                    crafting_station,
                },
            )),
            smithing: None,
        }
    }

    fn item_slot_display(item_id: i32) -> bbb_protocol::packets::SlotDisplaySummary {
        let mut raw_payload = Encoder::new();
        raw_payload.write_var_i32(VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID);
        raw_payload.write_var_i32(item_id);
        bbb_protocol::packets::SlotDisplaySummary {
            display_type_id: VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID,
            raw_payload: raw_payload.into_inner(),
            item_stack: Some(item_stack(item_id, 1)),
            tag: None,
        }
    }

    fn empty_slot_display() -> bbb_protocol::packets::SlotDisplaySummary {
        let mut raw_payload = Encoder::new();
        raw_payload.write_var_i32(VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID);
        bbb_protocol::packets::SlotDisplaySummary {
            display_type_id: VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID,
            raw_payload: raw_payload.into_inner(),
            item_stack: None,
            tag: None,
        }
    }
}
