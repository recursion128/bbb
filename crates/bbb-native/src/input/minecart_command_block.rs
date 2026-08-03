use bbb_control::NetCounters;
use bbb_net::NetCommand;
use bbb_renderer::hud_minecart_command_block_editor_layout;
use bbb_world::WorldStore;
use tokio::sync::mpsc;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton},
};

use super::{
    maybe_queue_command_suggestion_request, physical_position_floor_i32, release_active_input,
    set_chat_entry_cursor, submit_chat_entry, text_edit, ChatEntryOwner, ChatEntryState,
    ClientInputState, CommandSuggestionTextMeasurer,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct MinecartCommandBlockEditorInputState {
    pub(super) open_count: usize,
    pub(super) authoritative_revision: usize,
    pub(super) entity_id: i32,
    pub(super) track_output: bool,
    pub(super) previous_output: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MinecartCommandBlockEditorHudState {
    pub(crate) track_output: bool,
    pub(crate) previous_output: String,
    pub(crate) mouse_position: Option<(i32, i32)>,
}

impl ClientInputState {
    pub(crate) fn minecart_command_block_editor_is_active(&self) -> bool {
        self.chat_entry
            .as_ref()
            .is_some_and(|entry| matches!(entry.owner, ChatEntryOwner::MinecartCommandBlock(_)))
    }

    pub(crate) fn minecart_command_block_editor_hud_state(
        &self,
    ) -> Option<MinecartCommandBlockEditorHudState> {
        let entry = self.chat_entry.as_ref()?;
        let ChatEntryOwner::MinecartCommandBlock(editor) = &entry.owner else {
            return None;
        };
        Some(MinecartCommandBlockEditorHudState {
            track_output: editor.track_output,
            previous_output: editor.previous_output.clone(),
            mouse_position: entry.mouse_position,
        })
    }

    pub(crate) fn sync_minecart_command_block_editor(
        &mut self,
        world: &mut WorldStore,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
    ) {
        if world.dialog_screen_is_open() {
            return;
        }
        let replaced_by_another_screen = world.open_container_id().is_some()
            || world.current_book().is_some()
            || world.advancements_screen_is_open()
            || world.stats_screen_is_open()
            || world.social_interactions_screen().is_some()
            || world.game_rules_screen().is_some()
            || world.command_block_editor().is_some()
            || world.structure_block_editor().is_some()
            || world.jigsaw_block_editor().is_some()
            || world.test_block_editor().is_some()
            || world.test_instance_block_editor().is_some()
            || self.sign_editor_is_active_or_pending(world)
            || self.debug_options_screen_is_open()
            || self.debug_pause_screen_is_open();
        if replaced_by_another_screen {
            world.close_minecart_command_block_editor();
        }
        let Some(authoritative) = world.minecart_command_block_editor().cloned() else {
            if self.minecart_command_block_editor_is_active() {
                self.chat_entry = None;
            }
            return;
        };
        let open_count = world.minecart_command_block_editor_open_count();
        if let Some(editor) = self
            .chat_entry
            .as_mut()
            .and_then(|entry| match &mut entry.owner {
                ChatEntryOwner::MinecartCommandBlock(editor) => Some(editor),
                ChatEntryOwner::Chat(_) | ChatEntryOwner::CommandBlock(_) => None,
            })
        {
            if editor.open_count == open_count && editor.entity_id == authoritative.entity_id {
                editor.authoritative_revision = authoritative.authoritative_revision;
                editor.track_output = authoritative.data.track_output;
                editor.previous_output = authoritative
                    .data
                    .last_output
                    .as_ref()
                    .map(|output| output.text().to_string())
                    .unwrap_or_default();
                return;
            }
        }

        release_active_input(self, world, counters, net_commands);
        self.interrupt_chat_screen();
        self.structure_block_editor = None;
        self.jigsaw_block_editor = None;
        self.test_block_editor = None;
        self.test_instance_block_editor = None;
        world.close_command_block_editor();
        world.close_structure_block_editor();
        world.close_jigsaw_block_editor();
        world.close_test_block_editor();
        world.close_test_instance_block_editor();

        let cursor = text_edit::char_len(&authoritative.data.command);
        self.chat_entry = Some(ChatEntryState {
            text: authoritative.data.command,
            cursor,
            selection: cursor,
            owner: ChatEntryOwner::MinecartCommandBlock(MinecartCommandBlockEditorInputState {
                open_count,
                authoritative_revision: authoritative.authoritative_revision,
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
        });
        self.chat_scroll_lines = 0;
        maybe_queue_command_suggestion_request(self, counters, world, net_commands);
    }

    pub(crate) fn handle_minecart_command_block_editor_mouse_input(
        &mut self,
        world: &mut WorldStore,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        button: MouseButton,
        state: ElementState,
        cursor_position: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
        measurer: &impl CommandSuggestionTextMeasurer,
    ) -> bool {
        if !self.minecart_command_block_editor_is_active() {
            return false;
        }
        if let Some(entry) = self.chat_entry.as_mut() {
            entry.mouse_position = cursor_position.and_then(physical_position_floor_i32);
        }
        if !matches!((button, state), (MouseButton::Left, ElementState::Pressed)) {
            return true;
        }
        let Some((mouse_x, mouse_y)) = self
            .chat_entry
            .as_ref()
            .and_then(|entry| entry.mouse_position)
        else {
            return true;
        };
        let layout = hud_minecart_command_block_editor_layout(surface_size);
        if layout.command_input.contains(mouse_x, mouse_y) {
            let Some(entry) = self.chat_entry.as_mut() else {
                return true;
            };
            let display_start = measurer.command_suggestion_display_start(
                &entry.text,
                entry.cursor,
                layout.command_input.width.saturating_sub(8),
            );
            let text_offset = u32::try_from(
                mouse_x
                    .saturating_sub(layout.command_input.x)
                    .saturating_sub(4)
                    .max(0),
            )
            .unwrap_or_default();
            let cursor = measurer.command_suggestion_cursor_for_width(
                &entry.text,
                display_start,
                text_offset,
            );
            set_chat_entry_cursor(entry, cursor);
            return true;
        }
        if layout.cancel.contains(mouse_x, mouse_y) {
            self.chat_entry = None;
            self.chat_scroll_lines = 0;
            world.close_minecart_command_block_editor();
            return true;
        }
        if layout.done.contains(mouse_x, mouse_y) {
            submit_chat_entry(self, counters, world, net_commands);
            return true;
        }
        let Some(editor) = self
            .chat_entry
            .as_mut()
            .and_then(|entry| match &mut entry.owner {
                ChatEntryOwner::MinecartCommandBlock(editor) => Some(editor),
                ChatEntryOwner::Chat(_) | ChatEntryOwner::CommandBlock(_) => None,
            })
        else {
            return true;
        };
        if layout.track_output.contains(mouse_x, mouse_y) {
            editor.track_output = !editor.track_output;
            world.set_minecart_command_block_editor_track_output(editor.track_output);
        }
        true
    }
}
