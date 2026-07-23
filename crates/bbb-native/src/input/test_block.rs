use bbb_control::NetCounters;
use bbb_net::NetCommand;
use bbb_protocol::packets::{SetTestBlock, TestBlockMode, MAX_TEST_BLOCK_MESSAGE_UTF16_UNITS};
use bbb_renderer::{hud_test_block_editor_layout, HudCommandBlockEditorRect};
use bbb_world::{BlockPos, WorldStore};
use tokio::sync::mpsc;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton},
    keyboard::KeyCode,
};

use super::{
    physical_position_floor_i32, protocol_block_pos_from_world, queue_set_test_block_command,
    release_active_input, text_edit, ClientInputState, CommandSuggestionTextMeasurer,
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct TestBlockMessageInputState {
    text: String,
    cursor: usize,
    selection: usize,
}

impl TestBlockMessageInputState {
    fn new(text: String) -> Self {
        let text = truncate_utf16(text);
        let cursor = text_edit::char_len(&text);
        Self {
            text,
            cursor,
            selection: cursor,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TestBlockEditorInputState {
    open_count: usize,
    pos: BlockPos,
    message: TestBlockMessageInputState,
    mode: TestBlockMode,
    mouse_position: Option<(i32, i32)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TestBlockEditorHudState {
    pub(crate) message: String,
    pub(crate) cursor: usize,
    pub(crate) selection: usize,
    pub(crate) mode: TestBlockMode,
    pub(crate) mouse_position: Option<(i32, i32)>,
}

impl TestBlockEditorInputState {
    fn packet(&self) -> SetTestBlock {
        SetTestBlock {
            pos: protocol_block_pos_from_world(self.pos),
            mode: self.mode,
            message: self.message.text.clone(),
        }
    }
}

impl ClientInputState {
    pub(crate) fn test_block_editor_is_active(&self) -> bool {
        self.test_block_editor.is_some()
    }

    pub(crate) fn test_block_editor_hud_state(&self) -> Option<TestBlockEditorHudState> {
        let editor = self.test_block_editor.as_ref()?;
        Some(TestBlockEditorHudState {
            message: editor.message.text.clone(),
            cursor: editor.message.cursor,
            selection: editor.message.selection,
            mode: editor.mode,
            mouse_position: editor.mouse_position,
        })
    }

    pub(crate) fn sync_test_block_editor(
        &mut self,
        world: &mut WorldStore,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
    ) {
        if world.dialog_screen_is_open() {
            return;
        }
        let replaced = world.open_container_id().is_some()
            || world.current_book().is_some()
            || world.advancements_screen_is_open()
            || world.stats_screen_is_open()
            || world.game_rules_screen().is_some()
            || self.sign_editor_is_active_or_pending(world)
            || self.debug_options_screen_is_open()
            || self.debug_pause_screen_is_open()
            || self.command_block_editor_is_active()
            || self.minecart_command_block_editor_is_active()
            || self.structure_block_editor_is_active()
            || self.jigsaw_block_editor_is_active()
            || self.test_instance_block_editor_is_active();
        if replaced {
            world.close_test_block_editor();
        }
        let Some(authoritative) = world.test_block_editor().cloned() else {
            self.test_block_editor = None;
            return;
        };
        let open_count = world.test_block_editor_open_count();
        if self
            .test_block_editor
            .as_ref()
            .is_some_and(|editor| editor.open_count == open_count)
        {
            return;
        }

        release_active_input(self, world, counters, net_commands);
        self.chat_entry = None;
        self.chat_scroll_lines = 0;
        self.structure_block_editor = None;
        self.jigsaw_block_editor = None;
        self.test_instance_block_editor = None;
        world.close_command_block_editor();
        world.close_minecart_command_block_editor();
        world.close_structure_block_editor();
        world.close_jigsaw_block_editor();
        world.close_test_instance_block_editor();
        self.test_block_editor = Some(TestBlockEditorInputState {
            open_count,
            pos: authoritative.pos,
            message: TestBlockMessageInputState::new(authoritative.data.message),
            mode: authoritative.data.mode,
            mouse_position: None,
        });
    }

    pub(crate) fn handle_test_block_editor_cursor_moved(
        &mut self,
        cursor_position: Option<PhysicalPosition<f64>>,
    ) -> bool {
        let Some(editor) = self.test_block_editor.as_mut() else {
            return false;
        };
        editor.mouse_position = cursor_position.and_then(physical_position_floor_i32);
        true
    }

    pub(crate) fn handle_test_block_editor_key(
        &mut self,
        world: &mut WorldStore,
        code: KeyCode,
        pressed: bool,
    ) -> bool {
        if world.dialog_screen_is_open() || self.test_block_editor.is_none() {
            return false;
        }
        if !pressed {
            return true;
        }
        if code == KeyCode::Escape {
            self.test_block_editor = None;
            world.close_test_block_editor();
            return true;
        }
        let control_down = self.control_down();
        let shift_down = self.shift_down();
        let editor = self
            .test_block_editor
            .as_mut()
            .expect("checked test block editor");
        if editor.mode == TestBlockMode::Start {
            return true;
        }
        let field = &mut editor.message;
        match code {
            KeyCode::KeyA if control_down && !shift_down => {
                field.cursor = text_edit::char_len(&field.text);
                field.selection = 0;
            }
            KeyCode::ArrowLeft => {
                let cursor = if control_down {
                    text_edit::word_position(&field.text, field.cursor, -1)
                } else {
                    field.cursor.saturating_sub(1)
                };
                move_cursor(field, cursor, shift_down);
            }
            KeyCode::ArrowRight => {
                let cursor = if control_down {
                    text_edit::word_position(&field.text, field.cursor, 1)
                } else {
                    (field.cursor + 1).min(text_edit::char_len(&field.text))
                };
                move_cursor(field, cursor, shift_down);
            }
            KeyCode::Home => move_cursor(field, 0, shift_down),
            KeyCode::End => {
                let length = text_edit::char_len(&field.text);
                move_cursor(field, length, shift_down);
            }
            KeyCode::Backspace => {
                if !delete_selection(field) {
                    delete_before(field, control_down);
                }
            }
            KeyCode::Delete => {
                if !delete_selection(field) {
                    delete_at(field, control_down);
                }
            }
            _ => {}
        }
        true
    }

    pub(crate) fn handle_test_block_editor_text(&mut self, world: &WorldStore, text: &str) -> bool {
        if world.dialog_screen_is_open() {
            return false;
        }
        let Some(editor) = self.test_block_editor.as_mut() else {
            return false;
        };
        if editor.mode != TestBlockMode::Start {
            insert_text(&mut editor.message, text);
        }
        true
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn handle_test_block_editor_mouse_input(
        &mut self,
        counters: &mut NetCounters,
        world: &mut WorldStore,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        button: MouseButton,
        state: ElementState,
        cursor_position: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
        measurer: &impl CommandSuggestionTextMeasurer,
    ) -> bool {
        let Some(editor) = self.test_block_editor.as_mut() else {
            return false;
        };
        editor.mouse_position = cursor_position.and_then(physical_position_floor_i32);
        if !matches!((button, state), (MouseButton::Left, ElementState::Pressed)) {
            return true;
        }
        let Some((mouse_x, mouse_y)) = editor.mouse_position else {
            return true;
        };
        let layout = hud_test_block_editor_layout(surface_size);
        if editor.mode != TestBlockMode::Start && layout.message.contains(mouse_x, mouse_y) {
            set_cursor_from_mouse(&mut editor.message, layout.message, mouse_x, measurer);
            return true;
        }
        if layout.mode.contains(mouse_x, mouse_y) {
            editor.mode = next_mode(editor.mode);
            return true;
        }
        if layout.cancel.contains(mouse_x, mouse_y) {
            self.test_block_editor = None;
            world.close_test_block_editor();
            return true;
        }
        if layout.done.contains(mouse_x, mouse_y) {
            let packet = editor.packet();
            queue_set_test_block_command(counters, net_commands, packet);
            self.test_block_editor = None;
            world.close_test_block_editor();
        }
        true
    }
}

fn next_mode(mode: TestBlockMode) -> TestBlockMode {
    match mode {
        TestBlockMode::Start => TestBlockMode::Log,
        TestBlockMode::Log => TestBlockMode::Fail,
        TestBlockMode::Fail => TestBlockMode::Accept,
        TestBlockMode::Accept => TestBlockMode::Start,
    }
}

fn truncate_utf16(text: String) -> String {
    let mut units = 0usize;
    text.chars()
        .take_while(|ch| {
            let next = units.saturating_add(ch.len_utf16());
            let keep = next <= MAX_TEST_BLOCK_MESSAGE_UTF16_UNITS;
            if keep {
                units = next;
            }
            keep
        })
        .collect()
}

fn delete_selection(field: &mut TestBlockMessageInputState) -> bool {
    if field.cursor == field.selection {
        return false;
    }
    let start = field.cursor.min(field.selection);
    let end = field.cursor.max(field.selection);
    let start_byte = text_edit::byte_index(&field.text, start);
    let end_byte = text_edit::byte_index(&field.text, end);
    field.text.replace_range(start_byte..end_byte, "");
    field.cursor = start;
    field.selection = start;
    true
}

fn delete_before(field: &mut TestBlockMessageInputState, control_down: bool) {
    if field.cursor == 0 {
        return;
    }
    if control_down {
        text_edit::remove_word_before_cursor(&mut field.text, &mut field.cursor);
    } else {
        let start = text_edit::byte_index(&field.text, field.cursor - 1);
        let end = text_edit::byte_index(&field.text, field.cursor);
        field.text.replace_range(start..end, "");
        field.cursor -= 1;
    }
    field.selection = field.cursor;
}

fn delete_at(field: &mut TestBlockMessageInputState, control_down: bool) {
    if field.cursor >= text_edit::char_len(&field.text) {
        return;
    }
    if control_down {
        text_edit::remove_word_at_cursor(&mut field.text, field.cursor);
    } else {
        let start = text_edit::byte_index(&field.text, field.cursor);
        let end = text_edit::byte_index(&field.text, field.cursor + 1);
        field.text.replace_range(start..end, "");
    }
    field.selection = field.cursor;
}

fn insert_text(field: &mut TestBlockMessageInputState, text: &str) {
    delete_selection(field);
    let mut insert = String::new();
    let mut units = field.text.encode_utf16().count();
    for ch in text.chars().filter(|ch| is_edit_box_char(*ch)) {
        let next = units.saturating_add(ch.len_utf16());
        if next > MAX_TEST_BLOCK_MESSAGE_UTF16_UNITS {
            break;
        }
        insert.push(ch);
        units = next;
    }
    if insert.is_empty() {
        return;
    }
    let byte = text_edit::byte_index(&field.text, field.cursor);
    field.text.insert_str(byte, &insert);
    field.cursor += text_edit::char_len(&insert);
    field.selection = field.cursor;
}

fn is_edit_box_char(ch: char) -> bool {
    ch >= ' ' && ch != '\u{7f}' && ch != '\u{a7}'
}

fn move_cursor(field: &mut TestBlockMessageInputState, cursor: usize, selecting: bool) {
    field.cursor = cursor.min(text_edit::char_len(&field.text));
    if !selecting {
        field.selection = field.cursor;
    }
}

fn set_cursor_from_mouse(
    field: &mut TestBlockMessageInputState,
    rect: HudCommandBlockEditorRect,
    mouse_x: i32,
    measurer: &impl CommandSuggestionTextMeasurer,
) {
    let display_start = measurer.command_suggestion_display_start(
        &field.text,
        field.cursor,
        rect.width.saturating_sub(8),
    );
    let text_offset =
        u32::try_from(mouse_x.saturating_sub(rect.x).saturating_sub(4).max(0)).unwrap_or_default();
    field.cursor =
        measurer.command_suggestion_cursor_for_width(&field.text, display_start, text_offset);
    field.selection = field.cursor;
}
