use bbb_protocol::packets::{
    DialogHolder, InteractionHand, MountScreenOpen as ProtocolMountScreenOpen,
    OpenBook as ProtocolOpenBook, OpenSignEditor as ProtocolOpenSignEditor,
    PlaceGhostRecipe as ProtocolPlaceGhostRecipe, PongResponse as ProtocolPongResponse,
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
        self.client_ui.current_dialog = Some(DialogState::from_packet(packet));
    }

    pub fn apply_code_of_conduct(&mut self, text: String) {
        self.counters.code_of_conduct_packets += 1;
        self.counters.last_code_of_conduct_len = text.len();
        self.client_ui.last_code_of_conduct = Some(CodeOfConductState { text });
    }

    pub fn apply_mount_screen_open(&mut self, packet: ProtocolMountScreenOpen) {
        self.counters.mount_screen_open_packets += 1;
        self.client_ui.last_mount_screen = Some(MountScreenState {
            container_id: packet.container_id,
            inventory_columns: packet.inventory_columns,
            entity_id: packet.entity_id,
        });
    }

    pub fn apply_open_book(&mut self, packet: ProtocolOpenBook) {
        self.counters.open_book_packets += 1;
        self.client_ui.last_open_book = Some(OpenBookState {
            hand: interaction_hand_name(packet.hand).to_string(),
        });
    }

    pub fn apply_open_sign_editor(&mut self, packet: ProtocolOpenSignEditor) {
        self.counters.open_sign_editor_packets += 1;
        self.client_ui.last_open_sign_editor = Some(OpenSignEditorState {
            pos: protocol_block_pos(packet.pos),
            is_front_text: packet.is_front_text,
        });
    }

    pub fn apply_place_ghost_recipe(&mut self, packet: ProtocolPlaceGhostRecipe) {
        self.counters.ghost_recipe_packets += 1;
        self.client_ui.last_ghost_recipe = Some(GhostRecipeState {
            container_id: packet.container_id,
            recipe_display_type_id: packet.recipe_display_type.id(),
            recipe_display_type: packet.recipe_display_type.as_str().to_string(),
            recipe_display_body_len: packet.recipe_display_body.len(),
        });
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

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{BlockPos as ProtocolBlockPos, DialogHolder, InteractionHand};

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

        store.apply_code_of_conduct("Keep the server friendly.".to_string());

        assert_eq!(
            store.last_code_of_conduct(),
            Some(&CodeOfConductState {
                text: "Keep the server friendly.".to_string(),
            })
        );
        assert_eq!(store.counters().code_of_conduct_packets, 1);
        assert_eq!(
            store.counters().last_code_of_conduct_len,
            "Keep the server friendly.".len()
        );
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
    fn tracks_client_ui_pong_response() {
        let mut store = WorldStore::new();

        store.apply_pong_response(ProtocolPongResponse { time: 123456789 });

        assert_eq!(
            store.last_pong_response(),
            Some(&PongResponseState { time: 123456789 })
        );
        assert_eq!(store.counters().pong_response_packets, 1);
    }

    #[test]
    fn tracks_client_ui_ghost_recipe_request() {
        let mut store = WorldStore::new();

        store.apply_place_ghost_recipe(ProtocolPlaceGhostRecipe {
            container_id: 9,
            recipe_display_type: bbb_protocol::packets::RecipeDisplayType::Stonecutter,
            recipe_display_body: vec![1, 2, 3],
        });

        assert_eq!(
            store.last_ghost_recipe(),
            Some(&GhostRecipeState {
                container_id: 9,
                recipe_display_type_id: 3,
                recipe_display_type: "stonecutter".to_string(),
                recipe_display_body_len: 3,
            })
        );
        assert_eq!(store.counters().ghost_recipe_packets, 1);
    }
}
