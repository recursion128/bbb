use bbb_protocol::packets::{
    DialogHolder, InteractionHand, ItemStackSummary as ProtocolItemStackSummary,
    MountScreenOpen as ProtocolMountScreenOpen, OpenBook as ProtocolOpenBook,
    OpenSignEditor as ProtocolOpenSignEditor, PlaceGhostRecipe as ProtocolPlaceGhostRecipe,
    PongResponse as ProtocolPongResponse, RecipeDisplaySummary as ProtocolRecipeDisplaySummary,
    ShowDialog as ProtocolShowDialog,
};
use serde::{Deserialize, Serialize};

use crate::{protocol_block_pos, BlockPos, WorldStore};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientUiState {
    #[serde(default)]
    pub low_disk_space_warning_count: usize,
    #[serde(default)]
    pub current_dialog: Option<DialogState>,
    #[serde(default)]
    pub current_book: Option<BookScreenState>,
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
pub struct DialogState {
    pub holder_kind: String,
    pub registry_id: Option<i32>,
    pub raw_dialog_payload_len: usize,
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
        self.client_ui.current_dialog = None;
    }

    pub fn apply_show_dialog(&mut self, packet: ProtocolShowDialog) {
        self.counters.show_dialog_packets += 1;
        self.client_ui.current_book = None;
        self.client_ui.current_dialog = Some(DialogState::from_packet(packet));
    }

    pub fn apply_code_of_conduct(&mut self, text: String) {
        self.counters.code_of_conduct_packets += 1;
        self.counters.last_code_of_conduct_len = text.len();
        let text_hash = code_of_conduct_text_hash(&text);
        self.client_ui.last_code_of_conduct = Some(CodeOfConductState { text, text_hash });
    }

    pub fn apply_mount_screen_open(&mut self, packet: ProtocolMountScreenOpen) {
        self.counters.mount_screen_open_packets += 1;
        self.client_ui.current_book = None;
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
            self.client_ui.current_dialog = None;
            self.client_ui.current_book = Some(BookScreenState {
                hand,
                pages,
                current_page: 0,
            });
        }
    }

    pub fn apply_open_sign_editor(&mut self, packet: ProtocolOpenSignEditor) {
        self.counters.open_sign_editor_packets += 1;
        self.client_ui.current_book = None;
        self.client_ui.last_open_sign_editor = Some(OpenSignEditorState {
            pos: protocol_block_pos(packet.pos),
            is_front_text: packet.is_front_text,
        });
    }

    pub fn apply_place_ghost_recipe(&mut self, packet: ProtocolPlaceGhostRecipe) {
        self.counters.ghost_recipe_packets += 1;
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
}

impl DialogState {
    fn from_packet(packet: ProtocolShowDialog) -> Self {
        match packet.dialog {
            DialogHolder::Reference { registry_id } => Self {
                holder_kind: "reference".to_string(),
                registry_id: Some(registry_id),
                raw_dialog_payload_len: 0,
            },
            DialogHolder::Direct { raw_dialog_payload } => Self {
                holder_kind: "direct".to_string(),
                registry_id: None,
                raw_dialog_payload_len: raw_dialog_payload.len(),
            },
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
        .contains(&VANILLA_WRITABLE_BOOK_CONTENT_COMPONENT_ID)
        || !item.component_patch.writable_book_pages.is_empty()
    {
        return Some(item.component_patch.writable_book_pages.clone());
    }
    None
}

const VANILLA_WRITABLE_BOOK_CONTENT_COMPONENT_ID: i32 = 54;

pub fn code_of_conduct_text_hash(text: &str) -> i32 {
    text.encode_utf16().fold(0i32, |hash, unit| {
        hash.wrapping_mul(31).wrapping_add(i32::from(unit))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{
        BlockPos as ProtocolBlockPos, DataComponentPatchSummary, DialogHolder, InteractionHand,
        ItemStackSummary, SetPlayerInventory as ProtocolSetPlayerInventory,
        WrittenBookContentSummary,
    };

    #[test]
    fn tracks_client_ui_warnings_and_dialogs() {
        let mut store = WorldStore::new();

        store.apply_low_disk_space_warning();
        store.apply_show_dialog(ProtocolShowDialog {
            dialog: DialogHolder::Direct {
                raw_dialog_payload: vec![0xaa, 0xbb, 0xcc],
            },
        });

        assert_eq!(store.low_disk_space_warning_count(), 1);
        assert_eq!(
            store.current_dialog(),
            Some(&DialogState {
                holder_kind: "direct".to_string(),
                registry_id: None,
                raw_dialog_payload_len: 3,
            })
        );

        store.apply_show_dialog(ProtocolShowDialog {
            dialog: DialogHolder::Reference { registry_id: 11 },
        });

        assert_eq!(
            store.current_dialog(),
            Some(&DialogState {
                holder_kind: "reference".to_string(),
                registry_id: Some(11),
                raw_dialog_payload_len: 0,
            })
        );

        store.apply_clear_dialog();

        assert_eq!(store.current_dialog(), None);
        let counters = store.counters();
        assert_eq!(counters.low_disk_space_warnings, 1);
        assert_eq!(counters.show_dialog_packets, 2);
        assert_eq!(counters.clear_dialog_packets, 1);
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
    }

    #[test]
    fn open_book_without_book_access_records_request_without_screen() {
        let mut store = WorldStore::new();
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
            .push(VANILLA_WRITABLE_BOOK_CONTENT_COMPONENT_ID);
        stack.component_patch.writable_book_pages = pages.into_iter().map(str::to_string).collect();
        stack
    }

    #[test]
    fn tracks_client_ui_ghost_recipe_request() {
        let mut store = WorldStore::new();

        store.apply_place_ghost_recipe(ProtocolPlaceGhostRecipe {
            container_id: 9,
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
                container_id: 9,
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
    }

    #[test]
    fn clears_client_ui_ghost_recipe_without_packet_counter() {
        let mut store = WorldStore::new();

        store.apply_place_ghost_recipe(ProtocolPlaceGhostRecipe {
            container_id: 9,
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
