use crate::control::NetCounters;
use bbb_client_net::NetCommand;
use bbb_client_world::WorldStore;
use bbb_protocol::packets::MAX_COMMAND_BLOCK_COMMAND_UTF16_UNITS;
use tokio::sync::mpsc;

use super::{
    maybe_queue_command_suggestion_request, release_active_input, screen::Screen,
    text_field::TextField, ChatEntryState, ClientInputState, CommandEntryKind,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct MinecartCommandBlockEditorInputState {
    pub(super) entity_id: i32,
    pub(super) track_output: bool,
    pub(super) previous_output: String,
}

impl ClientInputState {
    pub(super) fn open_minecart_command_block_editor(
        &mut self,
        authoritative: bbb_client_world::MinecartCommandBlockEditorState,
        world: &mut WorldStore,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
    ) {
        release_active_input(self, world, counters, net_commands);
        let entry = ChatEntryState {
            edit: TextField::with_value(
                authoritative.data.command.units(),
                MAX_COMMAND_BLOCK_COMMAND_UTF16_UNITS,
            ),
            owner: CommandEntryKind::MinecartCommandBlock(MinecartCommandBlockEditorInputState {
                entity_id: authoritative.entity_id,
                track_output: authoritative.data.track_output,
                previous_output: authoritative
                    .data
                    .last_output
                    .as_ref()
                    .map(|output| output.text().to_string())
                    .unwrap_or_default(),
            }),
            ..ChatEntryState::default()
        };
        self.set_screen(Screen::CommandEntry(entry), counters, net_commands);
        self.reset_chat_scroll();
        maybe_queue_command_suggestion_request(self, counters, world, net_commands);
    }

    pub(super) fn refresh_minecart_command_block_editor(
        &mut self,
        authoritative: bbb_client_world::MinecartCommandBlockEditorState,
    ) {
        for entry in self.command_entries_mut() {
            if let CommandEntryKind::MinecartCommandBlock(editor) = &mut entry.owner {
                if editor.entity_id == authoritative.entity_id {
                    editor.track_output = authoritative.data.track_output;
                    editor.previous_output = authoritative
                        .data
                        .last_output
                        .as_ref()
                        .map(|output| output.text().to_string())
                        .unwrap_or_default();
                }
            }
        }
    }
}
