use bbb_protocol::{
    data_component_types::VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID,
    decode_nbt_summary,
    packets::{
        DialogHolder, InteractionHand, ItemStackSummary as ProtocolItemStackSummary,
        MountScreenOpen as ProtocolMountScreenOpen, OpenBook as ProtocolOpenBook,
        OpenSignEditor as ProtocolOpenSignEditor, PlaceGhostRecipe as ProtocolPlaceGhostRecipe,
        PongResponse as ProtocolPongResponse, RecipeDisplaySummary as ProtocolRecipeDisplaySummary,
        ShowDialog as ProtocolShowDialog,
    },
    NbtSummaryValue,
};
use serde::{Deserialize, Serialize};

use crate::{
    client::dialog::DialogState,
    entities::{is_vanilla_abstract_horse_type, is_vanilla_abstract_nautilus_type},
    protocol_block_pos, BlockPos, WorldStore,
};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientUiState {
    #[serde(default)]
    pub low_disk_space_warning_count: usize,
    #[serde(default)]
    pub current_dialog: Option<DialogState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dialog_previous_screen: Option<DialogPreviousScreenState>,
    #[serde(default)]
    pub current_book: Option<BookScreenState>,
    #[serde(default)]
    pub current_advancements_screen: bool,
    #[serde(default)]
    pub current_stats_screen: bool,
    #[serde(default)]
    pub stats_screen_loading: bool,
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
    Book { screen: BookScreenState },
    Advancements,
    Stats,
}

impl ClientUiState {
    fn show_dialog(&mut self, dialog: DialogState) {
        if self.current_dialog.is_none() {
            self.dialog_previous_screen = if let Some(screen) = self.current_book.take() {
                Some(DialogPreviousScreenState::Book { screen })
            } else if std::mem::take(&mut self.current_advancements_screen) {
                Some(DialogPreviousScreenState::Advancements)
            } else if std::mem::take(&mut self.current_stats_screen) {
                Some(DialogPreviousScreenState::Stats)
            } else {
                None
            };
        }
        self.current_dialog = Some(dialog);
    }

    fn clear_dialog(&mut self) -> bool {
        if self.current_dialog.take().is_none() {
            return false;
        }
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
            None => {}
        }
        true
    }

    pub(crate) fn replace_dialog(&mut self) {
        self.current_dialog = None;
        self.dialog_previous_screen = None;
    }
}

const DIALOG_REGISTRY: &str = "minecraft:dialog";

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
pub struct OpenBookState {
    pub hand: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookScreenState {
    pub hand: String,
    #[serde(default)]
    pub pages: Vec<String>,
    #[serde(default)]
    pub current_page: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenSignEditorState {
    pub pos: BlockPos,
    pub is_front_text: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GhostRecipeState {
    pub container_id: i32,
    pub recipe_display_type_id: i32,
    pub recipe_display_type: String,
    pub recipe_display_body_len: usize,
    #[serde(default)]
    pub recipe_display: Option<ProtocolRecipeDisplaySummary>,
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

    pub fn apply_open_book(&mut self, packet: ProtocolOpenBook) {
        self.counters.open_book_packets += 1;
        let hand = interaction_hand_name(packet.hand).to_string();
        self.client_ui.last_open_book = Some(OpenBookState { hand: hand.clone() });
        if let Some(pages) = self.open_book_pages_from_hand(packet.hand) {
            self.client_ui.replace_dialog();
            self.client_ui.current_advancements_screen = false;
            self.client_ui.current_stats_screen = false;
            self.client_ui.current_book = Some(BookScreenState {
                hand,
                pages,
                current_page: 0,
            });
        }
    }

    pub fn apply_open_sign_editor(&mut self, packet: ProtocolOpenSignEditor) {
        self.counters.open_sign_editor_packets += 1;
        let pos = protocol_block_pos(packet.pos);
        if !self.has_sign_block_entity_at(pos) {
            self.counters.open_sign_editor_updates_ignored += 1;
            return;
        }
        self.counters.open_sign_editor_updates_applied += 1;
        self.client_ui.replace_dialog();
        self.client_ui.current_book = None;
        self.client_ui.current_advancements_screen = false;
        self.client_ui.current_stats_screen = false;
        self.client_ui.last_open_sign_editor = Some(OpenSignEditorState {
            pos,
            is_front_text: packet.is_front_text,
        });
    }

    pub fn apply_place_ghost_recipe(&mut self, packet: ProtocolPlaceGhostRecipe) {
        self.counters.ghost_recipe_packets += 1;
        if self.recipe_update_listener_container_id() != Some(packet.container_id) {
            self.counters.ghost_recipe_updates_ignored += 1;
            return;
        }
        self.counters.ghost_recipe_updates_applied += 1;
        let recipe_display = packet.recipe_display;
        self.client_ui.last_ghost_recipe = Some(GhostRecipeState {
            container_id: packet.container_id,
            recipe_display_type_id: recipe_display.display_type.id(),
            recipe_display_type: recipe_display.display_type.as_str().to_string(),
            recipe_display_body_len: recipe_display.raw_body.len(),
            recipe_display: Some(recipe_display),
        });
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

    pub fn current_dialog(&self) -> Option<&DialogState> {
        self.client_ui.current_dialog.as_ref()
    }

    pub fn current_book(&self) -> Option<&BookScreenState> {
        self.client_ui.current_book.as_ref()
    }

    pub fn advancements_screen_is_open(&self) -> bool {
        self.client_ui.current_advancements_screen
    }

    pub fn open_advancements_screen(&mut self) -> bool {
        let was_open = self.client_ui.current_advancements_screen;
        self.client_ui.replace_dialog();
        self.client_ui.current_book = None;
        self.client_ui.current_stats_screen = false;
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
        self.client_ui.replace_dialog();
        self.client_ui.current_book = None;
        self.client_ui.current_advancements_screen = false;
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

    pub fn close_current_book(&mut self) -> bool {
        self.client_ui.current_book.take().is_some()
    }

    pub fn set_current_book_page(&mut self, page: usize) -> bool {
        let Some(book) = &mut self.client_ui.current_book else {
            return false;
        };
        let max_page = book.pages.len().saturating_sub(1);
        let page = page.min(max_page);
        if page == book.current_page {
            return false;
        }
        book.current_page = page;
        true
    }

    pub fn turn_current_book_page(&mut self, delta: i32) -> bool {
        let Some(book) = &self.client_ui.current_book else {
            return false;
        };
        let current = book.current_page as i32;
        let max_page = book.pages.len().saturating_sub(1) as i32;
        let page = current.saturating_add(delta).clamp(0, max_page);
        self.set_current_book_page(page as usize)
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

    pub fn last_open_book(&self) -> Option<&OpenBookState> {
        self.client_ui.last_open_book.as_ref()
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

    fn open_book_pages_from_hand(&self, hand: InteractionHand) -> Option<Vec<String>> {
        self.local_item_in_hand(hand).and_then(book_pages_from_item)
    }

    fn dialog_state_from_packet(&self, packet: ProtocolShowDialog) -> DialogState {
        match packet.dialog {
            DialogHolder::Reference { registry_id } => {
                let Some(entry) = self.registry_entry_by_holder_id(DIALOG_REGISTRY, registry_id)
                else {
                    return DialogState::unresolved_reference(
                        registry_id,
                        None,
                        format!("missing {DIALOG_REGISTRY} holder {registry_id}"),
                    );
                };
                let resource_id = entry.id.clone();
                let Some(raw_data) = entry.raw_data() else {
                    return DialogState::unresolved_reference(
                        registry_id,
                        Some(resource_id.clone()),
                        format!("dialog registry entry {resource_id} has no synchronized payload"),
                    );
                };
                match decode_nbt_summary(raw_data) {
                    Ok(Some(dialog @ NbtSummaryValue::Compound(_))) => {
                        DialogState::reference(registry_id, resource_id, dialog, &self.registries)
                    }
                    Ok(Some(_)) => DialogState::unresolved_reference(
                        registry_id,
                        Some(resource_id.clone()),
                        format!("dialog registry entry {resource_id} root is not a compound tag"),
                    ),
                    Ok(None) => DialogState::unresolved_reference(
                        registry_id,
                        Some(resource_id.clone()),
                        format!("dialog registry entry {resource_id} is an end tag"),
                    ),
                    Err(error) => DialogState::unresolved_reference(
                        registry_id,
                        Some(resource_id.clone()),
                        format!("invalid dialog registry entry {resource_id}: {error}"),
                    ),
                }
            }
            DialogHolder::Direct { dialog } => DialogState::direct(dialog, &self.registries),
            DialogHolder::ContextFree { dialog } => DialogState::context_free(dialog),
        }
    }
}

fn interaction_hand_name(hand: InteractionHand) -> &'static str {
    match hand {
        InteractionHand::MainHand => "main_hand",
        InteractionHand::OffHand => "off_hand",
    }
}

fn book_pages_from_item(item: &ProtocolItemStackSummary) -> Option<Vec<String>> {
    if let Some(written) = &item.component_patch.written_book {
        return Some(written.pages.clone());
    }
    if item
        .component_patch
        .added_type_ids
        .contains(&VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID)
        || !item.component_patch.writable_book_pages.is_empty()
    {
        return Some(item.component_patch.writable_book_pages.clone());
    }
    None
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
    use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_HORSE_ID, VANILLA_ENTITY_TYPE_PLAYER_ID};
    use bbb_protocol::packets::{
        AddEntity as ProtocolAddEntity, BlockPos as ProtocolBlockPos, DataComponentPatchSummary,
        DialogHolder, InteractionHand, ItemStackSummary, OpenScreen as ProtocolOpenScreen,
        SetPlayerInventory as ProtocolSetPlayerInventory, Vec3d as ProtocolVec3d,
        WrittenBookContentSummary,
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
    fn dialog_clear_restores_world_owned_previous_screen() {
        let previous_book = BookScreenState {
            hand: "main_hand".to_string(),
            pages: vec!["First".to_string(), "Second".to_string()],
            current_page: 1,
        };
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
            menu_type_id: 0,
            title: "Container".to_string(),
            title_styled: Vec::new(),
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
            })
        );

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

        assert_eq!(
            store.current_book(),
            Some(&BookScreenState {
                hand: "main_hand".to_string(),
                pages: vec!["First".to_string(), "Second".to_string()],
                current_page: 0,
            })
        );
        assert!(store.turn_current_book_page(1));
        assert_eq!(store.current_book().unwrap().current_page, 1);
        assert!(!store.turn_current_book_page(1));
        assert_eq!(store.current_book().unwrap().current_page, 1);
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

        assert_eq!(
            store.current_book(),
            Some(&BookScreenState {
                hand: "off_hand".to_string(),
                pages: vec!["Draft page".to_string()],
                current_page: 0,
            })
        );
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
                sign_text: Some(crate::SignBlockEntityTextState::default()),
                vault_shared_data: None,
                decorated_pot_sherds: None,
                banner_patterns: None,
                end_gateway: None,
                spawner: None,
            }],
            light: crate::LightData::default(),
        });
    }

    #[test]
    fn tracks_client_ui_ghost_recipe_request() {
        let mut store = WorldStore::new();
        assert!(store.open_local_inventory());

        store.apply_place_ghost_recipe(ProtocolPlaceGhostRecipe {
            container_id: 0,
            recipe_display: bbb_protocol::packets::RecipeDisplaySummary {
                display_type: bbb_protocol::packets::RecipeDisplayType::Stonecutter,
                raw_body: vec![3, 4, 100, 4, 101, 4, 102],
                crafting: None,
                furnace: None,
            },
        });

        assert_eq!(
            store.last_ghost_recipe(),
            Some(&GhostRecipeState {
                container_id: 0,
                recipe_display_type_id: 3,
                recipe_display_type: "stonecutter".to_string(),
                recipe_display_body_len: 7,
                recipe_display: Some(bbb_protocol::packets::RecipeDisplaySummary {
                    display_type: bbb_protocol::packets::RecipeDisplayType::Stonecutter,
                    raw_body: vec![3, 4, 100, 4, 101, 4, 102],
                    crafting: None,
                    furnace: None,
                }),
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
            recipe_display: bbb_protocol::packets::RecipeDisplaySummary {
                display_type: bbb_protocol::packets::RecipeDisplayType::Stonecutter,
                raw_body: vec![3],
                crafting: None,
                furnace: None,
            },
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
            recipe_display: bbb_protocol::packets::RecipeDisplaySummary {
                display_type: bbb_protocol::packets::RecipeDisplayType::Stonecutter,
                raw_body: vec![3],
                crafting: None,
                furnace: None,
            },
        });

        assert!(store.clear_ghost_recipe());
        assert_eq!(store.last_ghost_recipe(), None);
        assert!(!store.clear_ghost_recipe());
        assert_eq!(store.counters().ghost_recipe_packets, 1);
    }
}
