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
        DialogDataState, DialogState, DialogTextState,
    },
    client::inventory::{RecipeUpdateListenerContext, RecipeUpdateListenerLayout},
    entities::{is_vanilla_abstract_horse_type, is_vanilla_abstract_nautilus_type},
    for_each_shaped_recipe_grid_slot,
    recipes::slot_display_holders_are_resolved,
    BlockPos, ClientSystemToastEvent, WorldStore,
};

const MAX_GHOST_RECIPE_CANDIDATES: usize = 65_536;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientUiState {
    #[serde(default)]
    pub low_disk_space_warning_count: usize,
    #[serde(default)]
    pub resource_pack_load_failure_count: usize,
    #[serde(default)]
    pub narrator_toggle_toast_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub narrator_toggle_toast: Option<NarratorToggleToastState>,
    #[serde(default)]
    pub command_block_editor_open_count: usize,
    #[serde(default)]
    pub minecart_command_block_editor_open_count: usize,
    #[serde(default)]
    pub book_screen_generation: u64,
    #[serde(default)]
    pub game_event_screen_generation: u64,
    #[serde(default)]
    pub death_screen_generation: u64,
    #[serde(default)]
    pub last_code_of_conduct: Option<CodeOfConductState>,
    #[serde(default)]
    pub last_mount_screen: Option<MountScreenState>,
    #[serde(default)]
    pub last_open_book: Option<OpenBookState>,
    #[serde(skip)]
    screen_requests: Vec<ScreenRequest>,
    #[serde(default)]
    pub last_ghost_recipe: Option<GhostRecipeState>,
    #[serde(default)]
    pub last_pong_response: Option<PongResponseState>,
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

impl ClientUiState {
    pub(crate) fn push_screen_request(&mut self, request: ScreenRequest) {
        if matches!(request, ScreenRequest::RefreshResourcePackPrompt)
            && self
                .screen_requests
                .iter()
                .any(|existing| matches!(existing, ScreenRequest::RefreshResourcePackPrompt))
        {
            return;
        }
        self.screen_requests.push(request);
    }
    pub(crate) fn replace_screen(&mut self) {
        self.last_ghost_recipe = None;
    }
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
    pub dialog: DialogState,
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

/// A `Minecraft.setScreen` call of the packet listener, applied by the client
/// that owns the current screen, in packet order.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScreenRequest {
    ShowDialog(DialogState),
    RefreshResourcePackPrompt,
    ClearDialog,
    CommandBlockEditor(crate::CommandBlockEditorState),
    CommandBlockEditorData(crate::CommandBlockEditorState),
    CommandBlockEditorMode {
        pos: BlockPos,
        mode: Option<(bbb_protocol::packets::CommandBlockMode, bool)>,
    },
    CloseCommandBlockEditorInChunk(crate::ChunkPos),
    MinecartCommandBlockEditor(crate::MinecartCommandBlockEditorState),
    MinecartCommandBlockEditorData(crate::MinecartCommandBlockEditorState),
    CloseMinecartCommandBlockEditor(i32),
    SignEditor(OpenSignEditorState),
    /// `handleOpenBook`: a `BookViewScreen` over the held book.
    Book(BookScreenState),
    Death(crate::ClientDeathScreenState),
    GameEvent(ClientGameEventScreenState),
    /// `handleRespawn`: a `DeathScreen` (or its title confirmation) closes.
    CloseDeathScreen,
    /// Level teardown closes the screens bound to the old level.
    CloseLevelScreens,
    Container,
    CreativeInventory(bool),
    CloseContainer,
    /// Server container close or level loading replaces the current screen.
    CloseScreen,
    RefreshCreativeInventory,
    ReconcileCreativeInventory,
    /// `handleAwardStats`: an open `StatsScreen` stops loading.
    StatsUpdated,
    /// `GameRuleValues` arrived for a loading game rules screen.
    GameRuleValues,
    /// Respawn or lost permission closes the game rules screen.
    CloseGameRules,
    /// `handleTestInstanceBlockStatus`: an open `TestInstanceBlockEditScreen`
    /// shows the status.
    TestInstanceBlockStatus(crate::TestInstanceBlockStatusState),
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

/// One `GameNarrator.updateNarratorStatus` toast payload: the
/// `SystemToastId.NARRATOR_TOGGLE` slot shows `narrator.toast.enabled` with
/// the status name while the narrator narrates, and `narrator.toast.disabled`
/// with no detail line when the cycle lands on OFF. The
/// `options.narrator.notavailable` branch never publishes here: the Ctrl+B
/// hotkey requires an active narrator and the options UI is a non-goal, so no
/// caller can change the status of an inactive narrator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NarratorToggleToastState {
    pub enabled: bool,
    pub status_name: String,
}

impl WorldStore {
    pub fn apply_low_disk_space_warning(&mut self) {
        self.counters.low_disk_space_warnings += 1;
        self.client_ui.low_disk_space_warning_count += 1;
        self.push_system_toast_event(ClientSystemToastEvent::LowDiskSpace);
    }

    /// `SystemToast.addOrUpdate(NARRATOR_TOGGLE, ...)`: every status change
    /// replaces the retained payload and bumps the monotonic count the HUD
    /// toast animation watches.
    pub fn push_narrator_toggle_toast(&mut self, enabled: bool, status_name: &str) {
        self.client_ui.narrator_toggle_toast_count += 1;
        self.client_ui.narrator_toggle_toast = Some(NarratorToggleToastState {
            enabled,
            status_name: status_name.to_string(),
        });
        self.push_system_toast_event(ClientSystemToastEvent::NarratorToggle {
            enabled,
            status_name: status_name.to_string(),
        });
    }

    pub fn narrator_toggle_toast_count(&self) -> usize {
        self.client_ui.narrator_toggle_toast_count
    }

    pub fn narrator_toggle_toast(&self) -> Option<&NarratorToggleToastState> {
        self.client_ui.narrator_toggle_toast.as_ref()
    }

    /// Publishes one completed Vanilla `PACK_LOAD_FAILURE` system-toast update.
    ///
    /// Resource reload owns when recovery has completed; the renderer only
    /// projects this monotonic, session-scoped UI fact.
    pub fn apply_resource_pack_load_failure(&mut self) {
        self.client_ui.resource_pack_load_failure_count = self
            .client_ui
            .resource_pack_load_failure_count
            .saturating_add(1);
        self.push_system_toast_event(ClientSystemToastEvent::ResourcePackLoadFailure);
    }

    pub fn apply_clear_dialog(&mut self) {
        self.counters.clear_dialog_packets += 1;
        self.push_screen_request(ScreenRequest::ClearDialog);
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

    pub fn resolve_component_click_dialog(
        &self,
        dialog: &NbtSummaryValue,
        context_free: bool,
    ) -> Option<DialogState> {
        decode_component_click_dialog(dialog, (!context_free).then_some(&self.registries)).ok()
    }

    /// Resolves a lazy registry-referenced [`DialogState`] (for example from a
    /// `show_dialog` button action) into an active decoded dialog ready to show.
    pub fn resolve_dialog_state(&self, dialog: &DialogState) -> Option<DialogState> {
        if dialog.data.is_some() {
            return Some(dialog.clone());
        }
        if dialog.holder_kind == "reference" {
            let target = self.dialog_state_from_reference(dialog.registry_id?);
            target.data.as_ref()?;
            return Some(target);
        }
        None
    }
    pub fn resolve_dialog_list_entry(
        &self,
        current: &DialogState,
        index: usize,
    ) -> Option<DialogState> {
        let DialogDataState::DialogList { dialogs, .. } = current.data.as_ref()? else {
            return None;
        };
        let entry = dialogs.entries.get(index)?;
        let target = match entry.holder_kind.as_str() {
            "reference" => self.dialog_state_from_reference(entry.registry_id?),
            "direct" if current.holder_kind == "context_free" => {
                DialogState::context_free(entry.dialog.clone()?)
            }
            "direct" => DialogState::direct(entry.dialog.clone()?, &self.registries),
            _ => return None,
        };
        target.data.as_ref()?;
        Some(target)
    }
    pub fn apply_show_dialog(&mut self, packet: ProtocolShowDialog) {
        self.counters.show_dialog_packets += 1;
        let dialog = self.dialog_state_from_packet(packet);
        self.push_screen_request(ScreenRequest::ShowDialog(dialog));
    }
    pub fn apply_code_of_conduct(&mut self, text: String) {
        self.counters.code_of_conduct_packets += 1;
        self.counters.last_code_of_conduct_len = text.len();
        let text_hash = code_of_conduct_text_hash(&text);
        self.client_ui.last_code_of_conduct = Some(CodeOfConductState { text, text_hash });
    }

    pub fn apply_mount_screen_open(&mut self, packet: ProtocolMountScreenOpen) {
        const MAX_MOUNT_INVENTORY_COLUMNS: i32 = 64;
        self.counters.mount_screen_open_packets += 1;
        if !(0..=MAX_MOUNT_INVENTORY_COLUMNS).contains(&packet.inventory_columns) {
            return;
        }
        let Some(entity_type_id) = self.entity_type_id(packet.entity_id) else {
            return;
        };
        if !is_vanilla_abstract_horse_type(entity_type_id)
            && !is_vanilla_abstract_nautilus_type(entity_type_id)
        {
            return;
        }
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
        let pos = packet.pos;
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
        self.client_ui
            .screen_requests
            .push(ScreenRequest::SignEditor(OpenSignEditorState {
                pos,
                is_front_text: packet.is_front_text,
                lines,
                filtered_lines,
            }));
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
        let mut remaining_work = MAX_GHOST_RECIPE_CANDIDATES;
        for (slot_id, display, is_result) in assignments {
            let items = self
                .resolve_slot_display_stacks_with_budget(display, &mut remaining_work)
                .ok()?;
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
        self.push_screen_request(ScreenRequest::GameEvent(screen));
        screen
    }

    pub fn last_code_of_conduct(&self) -> Option<&CodeOfConductState> {
        self.client_ui.last_code_of_conduct.as_ref()
    }

    pub fn low_disk_space_warning_count(&self) -> usize {
        self.client_ui.low_disk_space_warning_count
    }

    /// A screen the client owns replaced the ones the world still owns.
    pub fn replace_screen(&mut self) {
        self.client_ui.replace_screen();
    }

    pub fn resource_pack_load_failure_count(&self) -> usize {
        self.client_ui.resource_pack_load_failure_count
    }

    pub fn last_mount_screen(&self) -> Option<&MountScreenState> {
        self.client_ui.last_mount_screen.as_ref()
    }

    pub fn screen_requests(&self) -> &[ScreenRequest] {
        &self.client_ui.screen_requests
    }

    pub(crate) fn push_screen_request(&mut self, request: ScreenRequest) {
        self.client_ui.push_screen_request(request);
    }

    pub fn take_screen_requests(&mut self) -> Vec<ScreenRequest> {
        std::mem::take(&mut self.client_ui.screen_requests)
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
                DialogState::reference(registry_id, resource_id, dialog, &self.registries, entry)
            }
            Err(error) => DialogState::unresolved_reference(registry_id, Some(resource_id), error),
        }
    }
}

pub use bbb_protocol::{code_of_conduct_text_hash, java_string_hash_code};

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID;
    use bbb_protocol::codec::Encoder;
    use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID;
    use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_HORSE_ID, VANILLA_ENTITY_TYPE_PLAYER_ID};
    use bbb_protocol::menu_types::{
        VANILLA_MENU_TYPE_BLAST_FURNACE_ID, VANILLA_MENU_TYPE_CRAFTING_ID,
        VANILLA_MENU_TYPE_FURNACE_ID, VANILLA_MENU_TYPE_SMOKER_ID,
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

    fn requested_dialog(store: &WorldStore) -> &DialogState {
        store
            .screen_requests()
            .iter()
            .rev()
            .find_map(|request| match request {
                ScreenRequest::ShowDialog(dialog) => Some(dialog),
                _ => None,
            })
            .unwrap()
    }
    mod dialogs;
    mod screens_and_recipes;

    #[test]
    fn tracks_client_ui_warnings_and_dialogs() {
        let mut store = WorldStore::new();

        store.apply_low_disk_space_warning();
        store.apply_resource_pack_load_failure();
        store.apply_show_dialog(ProtocolShowDialog {
            dialog: DialogHolder::Direct {
                dialog: NbtSummaryValue::Compound(vec![
                    NbtSummaryEntry {
                        name: "type".into(),
                        value: NbtSummaryValue::string("minecraft:notice"),
                    },
                    NbtSummaryEntry {
                        name: "title".into(),
                        value: NbtSummaryValue::string("Title"),
                    },
                ]),
            },
        });

        assert_eq!(store.low_disk_space_warning_count(), 1);
        assert_eq!(store.resource_pack_load_failure_count(), 1);
        let direct = requested_dialog(&store);
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

        let referenced = requested_dialog(&store);
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

        assert!(matches!(
            store.take_screen_requests().last(),
            Some(ScreenRequest::ClearDialog)
        ));
        let counters = store.counters();
        assert_eq!(counters.low_disk_space_warnings, 1);
        assert_eq!(counters.show_dialog_packets, 2);
        assert_eq!(counters.clear_dialog_packets, 1);
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
        let resolved = requested_dialog(&store);
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
        let stub = requested_dialog(&store);
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
        let malformed = requested_dialog(&store);
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
        let missing = requested_dialog(&store);
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
            requested_sign_editor(&store),
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
        let editor = requested_sign_editor(&store).unwrap();
        assert_eq!(editor.lines_for_filter(false)[0], "raw back");
        assert_eq!(editor.lines_for_filter(true)[0], "filtered back");

        let counters = store.counters();
        assert_eq!(counters.mount_screen_open_packets, 1);
        assert_eq!(counters.open_book_packets, 1);
        assert_eq!(counters.open_sign_editor_packets, 1);
        assert_eq!(counters.open_sign_editor_updates_applied, 1);
        assert_eq!(counters.open_sign_editor_updates_ignored, 0);
    }

    fn requested_book(store: &WorldStore) -> Option<&BookScreenState> {
        store
            .screen_requests()
            .iter()
            .rev()
            .find_map(|request| match request {
                ScreenRequest::Book(book) => Some(book),
                _ => None,
            })
    }

    fn requested_sign_editor(store: &WorldStore) -> Option<&OpenSignEditorState> {
        store
            .screen_requests()
            .iter()
            .rev()
            .find_map(|request| match request {
                ScreenRequest::SignEditor(editor) => Some(editor),
                _ => None,
            })
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
        assert_eq!(requested_sign_editor(&store), None);
        assert_eq!(store.counters().open_sign_editor_packets, 1);
        assert_eq!(store.counters().open_sign_editor_updates_applied, 0);
        assert_eq!(store.counters().open_sign_editor_updates_ignored, 1);

        let pos = BlockPos { x: 3, y: 64, z: -9 };
        insert_test_sign(&mut store, pos);
        store.apply_open_sign_editor(packet);
        assert_eq!(requested_sign_editor(&store).unwrap().pos, pos);
        assert!(matches!(
            store.take_screen_requests().last(),
            Some(ScreenRequest::SignEditor(_))
        ));
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

        store.apply_add_entity(test_entity(42, VANILLA_ENTITY_TYPE_HORSE_ID));
        store.apply_mount_screen_open(ProtocolMountScreenOpen {
            container_id: 11,
            inventory_columns: 5,
            entity_id: 42,
        });
        assert!(store.last_mount_screen().is_some());
        assert!(store.inventory().open_container.is_some());
        assert!(matches!(
            store.take_screen_requests().last(),
            Some(ScreenRequest::Container)
        ));
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
        assert_eq!(requested_book(&store), None);
        assert_eq!(store.counters().open_book_packets, 1);
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
            component_patch: DataComponentPatchSummary::default().into(),
        }
    }

    fn written_book_stack(pages: Vec<&str>) -> ItemStackSummary {
        let mut stack = item_stack(42, 1);
        let pages: Vec<String> = pages.into_iter().map(str::to_string).collect();
        let page_filters = vec![None; pages.len()];
        stack.component_patch.written_book = Some(WrittenBookContentSummary {
            title: "Guide".into(),
            title_filter: None,
            author: "Alex".into(),
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
        stack.component_patch.writable_book_pages = pages.into_iter().map(Into::into).collect();
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
            block_entities: std::sync::Arc::new(vec![crate::BlockEntityRecord::empty(
                pos.x.rem_euclid(16) as u8,
                i16::try_from(pos.y).unwrap(),
                pos.z.rem_euclid(16) as u8,
                VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID,
            )
            .with_raw_nbt(vec![0])
            .with_payload(crate::BlockEntityPayload::SignText(Box::new(
                crate::SignBlockEntityTextState {
                    front: test_sign_side("raw front", "filtered front"),
                    back: test_sign_side("raw back", "filtered back"),
                    is_waxed: false,
                },
            )))]),
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

        store.replace_screen();
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
