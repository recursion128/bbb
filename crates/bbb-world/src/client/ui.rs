use bbb_protocol::packets::{
    InteractionHand, MountScreenOpen as ProtocolMountScreenOpen, OpenBook as ProtocolOpenBook,
    OpenSignEditor as ProtocolOpenSignEditor,
};
use serde::{Deserialize, Serialize};

use crate::{protocol_block_pos, BlockPos, WorldStore};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientUiState {
    pub last_mount_screen: Option<MountScreenState>,
    pub last_open_book: Option<OpenBookState>,
    pub last_open_sign_editor: Option<OpenSignEditorState>,
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

impl WorldStore {
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

    pub fn client_ui(&self) -> &ClientUiState {
        &self.client_ui
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
    use bbb_protocol::packets::{BlockPos as ProtocolBlockPos, InteractionHand};

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
}
