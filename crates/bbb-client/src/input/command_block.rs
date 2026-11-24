//! Command block editor input and HUD state.

use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommandBlockEditorInputState {
    pub(super) pos: BlockPos,
    pub(crate) mode: CommandBlockMode,
    pub(crate) conditional: bool,
    pub(crate) automatic: bool,
    pub(super) track_output: bool,
    pub(super) previous_output: String,
    pub(crate) controls_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommandBlockEditorHudState<'a> {
    pub(crate) block: Option<&'a CommandBlockEditorInputState>,
    pub(crate) track_output: bool,
    pub(crate) previous_output: &'a str,
    pub(crate) mouse_position: Option<(i32, i32)>,
}

impl ClientInputState {
    pub(crate) fn command_block_editor_hud_state(&self) -> Option<CommandBlockEditorHudState<'_>> {
        let entry = self.screen.chat_entry()?;
        let (block, track_output, previous_output) = match &entry.owner {
            CommandEntryKind::CommandBlock(editor) => (
                Some(editor),
                editor.track_output,
                editor.previous_output.as_str(),
            ),
            CommandEntryKind::MinecartCommandBlock(editor) => {
                (None, editor.track_output, editor.previous_output.as_str())
            }
            CommandEntryKind::Chat(_) => return None,
        };
        Some(CommandBlockEditorHudState {
            block,
            track_output,
            previous_output,
            mouse_position: entry.mouse_position,
        })
    }

    pub(crate) fn command_block_editor_command_text(&self) -> Option<String> {
        let entry = self.screen.chat_entry()?;
        (!matches!(entry.owner, CommandEntryKind::Chat(_))).then(|| entry.edit.text())
    }

    pub(super) fn open_command_block_editor(
        &mut self,
        authoritative: bbb_client_world::CommandBlockEditorState,
        world: &mut WorldStore,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
    ) {
        release_active_input(self, world, counters, net_commands);
        self.set_screen(
            Screen::CommandEntry(command_block_entry(authoritative, false)),
            counters,
            net_commands,
        );
        self.reset_chat_scroll();
        maybe_queue_command_suggestion_request(self, counters, world, net_commands);
    }

    pub(super) fn refresh_command_block_editor(
        &mut self,
        authoritative: bbb_client_world::CommandBlockEditorState,
    ) {
        for entry in self.command_entries_mut() {
            if matches!(&entry.owner, CommandEntryKind::CommandBlock(editor) if editor.pos == authoritative.pos)
            {
                *entry = command_block_entry(authoritative.clone(), true);
            }
        }
    }

    pub(super) fn refresh_command_block_editor_mode(
        &mut self,
        pos: BlockPos,
        mode: Option<(CommandBlockMode, bool)>,
        world: &WorldStore,
    ) {
        let Some((mode, conditional)) = mode else {
            self.close_command_entry_if(
                |kind| matches!(kind, CommandEntryKind::CommandBlock(editor) if editor.pos == pos),
            );
            return;
        };
        for entry in self.command_entries_mut() {
            if let CommandEntryKind::CommandBlock(editor) = &entry.owner {
                if editor.pos == pos {
                    if let Some(data) = world.command_block_data_at(pos) {
                        let authoritative = bbb_client_world::CommandBlockEditorState {
                            pos,
                            data: data.clone(),
                            mode,
                            conditional,
                        };
                        *entry = command_block_entry(authoritative, editor.controls_enabled);
                    }
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn handle_command_block_editor_mouse_input(
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
        let extend = self.shift_down();
        let Some(entry) = self.screen.chat_entry_mut() else {
            return false;
        };
        let (layout, controls_enabled) = match &entry.owner {
            CommandEntryKind::CommandBlock(editor) => (
                hud_command_block_editor_layout(surface_size),
                editor.controls_enabled,
            ),
            CommandEntryKind::MinecartCommandBlock(_) => (
                bbb_client_render::hud_minecart_command_block_editor_layout(surface_size),
                true,
            ),
            CommandEntryKind::Chat(_) => return false,
        };
        entry.mouse_position = cursor_position.and_then(physical_position_floor_i32);
        if !matches!((button, state), (MouseButton::Left, ElementState::Pressed)) {
            return true;
        }
        let Some((mouse_x, mouse_y)) = entry.mouse_position else {
            return true;
        };
        if layout.command_input.contains(mouse_x, mouse_y) {
            entry.edit.click_bordered(
                layout.command_input,
                mouse_x,
                measurer.command_suggestion_glyphs(),
                extend,
            );
            return true;
        }
        if layout.cancel.contains(mouse_x, mouse_y) {
            self.screen.take_chat_entry();
            self.reset_chat_scroll();
            return true;
        }
        if !controls_enabled {
            return true;
        }
        if layout.done.contains(mouse_x, mouse_y) {
            submit_chat_entry(self, counters, world, net_commands);
            return true;
        }
        let editor = match &mut entry.owner {
            CommandEntryKind::CommandBlock(editor) => editor,
            CommandEntryKind::MinecartCommandBlock(editor) => {
                if layout.track_output.contains(mouse_x, mouse_y) {
                    editor.track_output = !editor.track_output;
                    world.set_minecart_command_block_track_output(
                        editor.entity_id,
                        editor.track_output,
                    );
                }
                return true;
            }
            CommandEntryKind::Chat(_) => unreachable!(),
        };
        if layout.mode.contains(mouse_x, mouse_y) {
            editor.mode = match editor.mode {
                CommandBlockMode::Sequence => CommandBlockMode::Auto,
                CommandBlockMode::Auto => CommandBlockMode::Redstone,
                CommandBlockMode::Redstone => CommandBlockMode::Sequence,
            };
        } else if layout.conditional.contains(mouse_x, mouse_y) {
            editor.conditional = !editor.conditional;
        } else if layout.automatic.contains(mouse_x, mouse_y) {
            editor.automatic = !editor.automatic;
        } else if layout.track_output.contains(mouse_x, mouse_y) {
            editor.track_output = !editor.track_output;
            world.set_command_block_track_output(editor.pos, editor.track_output);
        }
        true
    }
}

fn command_block_entry(
    authoritative: bbb_client_world::CommandBlockEditorState,
    controls_enabled: bool,
) -> ChatEntryState {
    ChatEntryState {
        edit: text_field::TextField::with_value(
            authoritative.data.command.units(),
            MAX_COMMAND_BLOCK_COMMAND_UTF16_UNITS,
        ),
        owner: CommandEntryKind::CommandBlock(CommandBlockEditorInputState {
            pos: authoritative.pos,
            mode: authoritative.mode,
            conditional: authoritative.conditional,
            automatic: authoritative.data.automatic,
            track_output: authoritative.data.track_output,
            previous_output: authoritative
                .data
                .last_output
                .as_ref()
                .map(|output| output.text().to_string())
                .unwrap_or_default(),
            controls_enabled,
        }),
        ..ChatEntryState::default()
    }
}
