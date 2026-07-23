use bbb_control::NetCounters;
use bbb_net::NetCommand;
use bbb_protocol::{
    normalize_resource_location,
    packets::{
        TestInstanceBlockAction, TestInstanceBlockActionKind, TestInstanceBlockRotation, Vec3i,
        MAX_TEST_INSTANCE_ID_UTF16_UNITS,
    },
    RichTextComponent,
};
use bbb_renderer::{hud_test_instance_block_editor_layout, HudCommandBlockEditorRect};
use bbb_world::{BlockPos, WorldStore};
use tokio::sync::mpsc;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton},
    keyboard::KeyCode,
};

use super::{
    physical_position_floor_i32, protocol_block_pos_from_world,
    queue_test_instance_block_action_command, release_active_input, text_edit, ClientInputState,
    CommandSuggestionTextMeasurer,
};

const SIZE_FIELD_UTF16_UNITS: usize = 15;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TestInstanceBlockEditorFocus {
    Id,
    SizeX,
    SizeY,
    SizeZ,
}

impl TestInstanceBlockEditorFocus {
    fn index(self) -> usize {
        match self {
            Self::Id => 0,
            Self::SizeX => 1,
            Self::SizeY => 2,
            Self::SizeZ => 3,
        }
    }

    fn from_index(index: usize) -> Self {
        match index % 4 {
            0 => Self::Id,
            1 => Self::SizeX,
            2 => Self::SizeY,
            _ => Self::SizeZ,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TestInstanceBlockEditorTextField {
    pub(super) text: String,
    pub(super) cursor: usize,
    pub(super) selection: usize,
    pub(super) max_utf16_units: usize,
}

impl TestInstanceBlockEditorTextField {
    fn new(text: String, max_utf16_units: usize) -> Self {
        let cursor = text_edit::char_len(&text);
        Self {
            text,
            cursor,
            selection: cursor,
            max_utf16_units,
        }
    }

    fn set_text(&mut self, text: String) {
        self.text = truncate_utf16(text, self.max_utf16_units);
        self.cursor = text_edit::char_len(&self.text);
        self.selection = self.cursor;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TestInstanceBlockEditorInputState {
    pub(super) open_count: usize,
    pub(super) authoritative_revision: usize,
    pub(super) response_revision: usize,
    pub(super) pos: BlockPos,
    pub(super) fields: [TestInstanceBlockEditorTextField; 4],
    pub(super) focus: TestInstanceBlockEditorFocus,
    pub(super) rotation: TestInstanceBlockRotation,
    pub(super) include_entities: bool,
    pub(super) error_message: Option<RichTextComponent>,
    pub(super) response_status: Option<RichTextComponent>,
    pub(super) local_invalid_id: bool,
    pub(super) mouse_position: Option<(i32, i32)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TestInstanceBlockEditorHudTextField {
    pub(crate) text: String,
    pub(crate) cursor: usize,
    pub(crate) selection: usize,
    pub(crate) max_utf16_units: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TestInstanceBlockEditorHudState {
    pub(crate) fields: [TestInstanceBlockEditorHudTextField; 4],
    pub(crate) focus_index: usize,
    pub(crate) rotation: TestInstanceBlockRotation,
    pub(crate) include_entities: bool,
    pub(crate) error_message: Option<RichTextComponent>,
    pub(crate) response_status: Option<RichTextComponent>,
    pub(crate) local_invalid_id: bool,
    pub(crate) mouse_position: Option<(i32, i32)>,
    pub(crate) save_enabled: bool,
}

impl TestInstanceBlockEditorInputState {
    fn field(&self) -> &TestInstanceBlockEditorTextField {
        &self.fields[self.focus.index()]
    }

    fn field_mut(&mut self) -> &mut TestInstanceBlockEditorTextField {
        &mut self.fields[self.focus.index()]
    }

    fn normalized_test(&self) -> Option<String> {
        normalize_resource_location(self.fields[0].text.clone()).ok()
    }

    fn size(&self) -> Vec3i {
        Vec3i {
            x: parse_size(&self.fields[1].text),
            y: parse_size(&self.fields[2].text),
            z: parse_size(&self.fields[3].text),
        }
    }

    fn action(&self, action: TestInstanceBlockActionKind) -> TestInstanceBlockAction {
        TestInstanceBlockAction {
            pos: protocol_block_pos_from_world(self.pos),
            action,
            test: self.normalized_test(),
            size: self.size(),
            rotation: self.rotation,
            ignore_entities: !self.include_entities,
        }
    }

    fn save_enabled(&self) -> bool {
        self.rotation == TestInstanceBlockRotation::None && self.normalized_test().is_some()
    }
}

impl ClientInputState {
    pub(crate) fn test_instance_block_editor_is_active(&self) -> bool {
        self.test_instance_block_editor.is_some()
    }

    pub(crate) fn test_instance_block_editor_hud_state(
        &self,
    ) -> Option<TestInstanceBlockEditorHudState> {
        let editor = self.test_instance_block_editor.as_ref()?;
        Some(TestInstanceBlockEditorHudState {
            fields: editor
                .fields
                .clone()
                .map(|field| TestInstanceBlockEditorHudTextField {
                    text: field.text,
                    cursor: field.cursor,
                    selection: field.selection,
                    max_utf16_units: field.max_utf16_units,
                }),
            focus_index: editor.focus.index(),
            rotation: editor.rotation,
            include_entities: editor.include_entities,
            error_message: editor.error_message.clone(),
            response_status: editor.response_status.clone(),
            local_invalid_id: editor.local_invalid_id,
            mouse_position: editor.mouse_position,
            save_enabled: editor.save_enabled(),
        })
    }

    pub(crate) fn sync_test_instance_block_editor(
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
            || self.sign_editor_is_active_or_pending(world)
            || self.debug_options_screen_is_open()
            || self.debug_pause_screen_is_open()
            || self.structure_block_editor_is_active()
            || self.jigsaw_block_editor_is_active()
            || self.test_block_editor_is_active()
            || self.command_block_editor_is_active()
            || self.minecart_command_block_editor_is_active();
        if replaced_by_another_screen {
            world.close_test_instance_block_editor();
        }
        let Some(authoritative) = world.test_instance_block_editor().cloned() else {
            self.test_instance_block_editor = None;
            return;
        };
        let open_count = world.test_instance_block_editor_open_count();
        let is_new_open = self
            .test_instance_block_editor
            .as_ref()
            .is_none_or(|editor| editor.open_count != open_count);
        if is_new_open {
            release_active_input(self, world, counters, net_commands);
            self.chat_entry = None;
            self.chat_scroll_lines = 0;
            self.jigsaw_block_editor = None;
            self.test_block_editor = None;
            world.close_command_block_editor();
            world.close_jigsaw_block_editor();
            world.close_test_block_editor();
            let test = authoritative.data.test.clone().unwrap_or_default();
            let mut editor = TestInstanceBlockEditorInputState {
                open_count,
                authoritative_revision: authoritative.authoritative_revision,
                response_revision: authoritative.response_revision,
                pos: authoritative.pos,
                fields: [
                    TestInstanceBlockEditorTextField::new(test, MAX_TEST_INSTANCE_ID_UTF16_UNITS),
                    TestInstanceBlockEditorTextField::new(
                        authoritative.data.size.x.to_string(),
                        SIZE_FIELD_UTF16_UNITS,
                    ),
                    TestInstanceBlockEditorTextField::new(
                        authoritative.data.size.y.to_string(),
                        SIZE_FIELD_UTF16_UNITS,
                    ),
                    TestInstanceBlockEditorTextField::new(
                        authoritative.data.size.z.to_string(),
                        SIZE_FIELD_UTF16_UNITS,
                    ),
                ],
                focus: TestInstanceBlockEditorFocus::Id,
                rotation: authoritative.data.rotation,
                include_entities: !authoritative.data.ignore_entities,
                error_message: authoritative.data.error_message.clone(),
                response_status: authoritative.response_status.clone(),
                local_invalid_id: false,
                mouse_position: None,
            };
            editor.local_invalid_id = editor.normalized_test().is_none();
            let init = editor.action(TestInstanceBlockActionKind::Init);
            self.test_instance_block_editor = Some(editor);
            queue_test_instance_block_action_command(counters, net_commands, init);
            return;
        }

        let Some(editor) = self.test_instance_block_editor.as_mut() else {
            return;
        };
        if editor.authoritative_revision != authoritative.authoritative_revision {
            editor.authoritative_revision = authoritative.authoritative_revision;
            editor.fields[0].set_text(authoritative.data.test.unwrap_or_default());
            editor.fields[1].set_text(authoritative.data.size.x.to_string());
            editor.fields[2].set_text(authoritative.data.size.y.to_string());
            editor.fields[3].set_text(authoritative.data.size.z.to_string());
            editor.rotation = authoritative.data.rotation;
            editor.include_entities = !authoritative.data.ignore_entities;
            editor.error_message = authoritative.data.error_message;
            editor.local_invalid_id = editor.normalized_test().is_none();
        }
        if editor.response_revision != authoritative.response_revision {
            editor.response_revision = authoritative.response_revision;
            editor.response_status = authoritative.response_status;
            editor.local_invalid_id = false;
            if let Some(size) = authoritative.response_size {
                editor.fields[1].set_text(size.x.to_string());
                editor.fields[2].set_text(size.y.to_string());
                editor.fields[3].set_text(size.z.to_string());
            }
        }
    }

    pub(crate) fn handle_test_instance_block_editor_cursor_moved(
        &mut self,
        cursor_position: Option<PhysicalPosition<f64>>,
    ) -> bool {
        let Some(editor) = self.test_instance_block_editor.as_mut() else {
            return false;
        };
        editor.mouse_position = cursor_position.and_then(physical_position_floor_i32);
        true
    }

    pub(crate) fn handle_test_instance_block_editor_key(
        &mut self,
        counters: &mut NetCounters,
        world: &mut WorldStore,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        code: KeyCode,
        pressed: bool,
    ) -> bool {
        if world.dialog_screen_is_open() || self.test_instance_block_editor.is_none() {
            return false;
        }
        if !pressed {
            return true;
        }
        if code == KeyCode::Escape {
            self.test_instance_block_editor = None;
            world.close_test_instance_block_editor();
            return true;
        }
        let control_down = self.control_down();
        let shift_down = self.shift_down();
        let mut id_changed = false;
        let editor = self
            .test_instance_block_editor
            .as_mut()
            .expect("checked test-instance editor");
        match code {
            KeyCode::Tab => {
                let index = editor.focus.index();
                editor.focus = TestInstanceBlockEditorFocus::from_index(if shift_down {
                    index.wrapping_add(3)
                } else {
                    index + 1
                });
            }
            KeyCode::KeyA if control_down && !shift_down => {
                let field = editor.field_mut();
                field.cursor = text_edit::char_len(&field.text);
                field.selection = 0;
            }
            KeyCode::ArrowLeft => {
                let field = editor.field_mut();
                let cursor = if control_down {
                    text_edit::word_position(&field.text, field.cursor, -1)
                } else {
                    field.cursor.saturating_sub(1)
                };
                move_cursor(field, cursor, shift_down);
            }
            KeyCode::ArrowRight => {
                let field = editor.field_mut();
                let cursor = if control_down {
                    text_edit::word_position(&field.text, field.cursor, 1)
                } else {
                    (field.cursor + 1).min(text_edit::char_len(&field.text))
                };
                move_cursor(field, cursor, shift_down);
            }
            KeyCode::Home => move_cursor(editor.field_mut(), 0, shift_down),
            KeyCode::End => {
                let length = text_edit::char_len(&editor.field().text);
                move_cursor(editor.field_mut(), length, shift_down);
            }
            KeyCode::Backspace => {
                let id_field = editor.focus == TestInstanceBlockEditorFocus::Id;
                let field = editor.field_mut();
                let changed = delete_selection(field) || delete_before(field, control_down);
                id_changed = id_field && changed;
            }
            KeyCode::Delete => {
                let id_field = editor.focus == TestInstanceBlockEditorFocus::Id;
                let field = editor.field_mut();
                let changed = delete_selection(field) || delete_at(field, control_down);
                id_changed = id_field && changed;
            }
            _ => {}
        }
        if id_changed {
            self.queue_test_instance_block_query(counters, net_commands);
        }
        true
    }

    pub(crate) fn handle_test_instance_block_editor_text(
        &mut self,
        counters: &mut NetCounters,
        world: &WorldStore,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        text: &str,
    ) -> bool {
        if world.dialog_screen_is_open() {
            return false;
        }
        let Some(editor) = self.test_instance_block_editor.as_mut() else {
            return false;
        };
        let id_field = editor.focus == TestInstanceBlockEditorFocus::Id;
        let changed = insert_text(editor.field_mut(), text);
        if changed && id_field {
            self.queue_test_instance_block_query(counters, net_commands);
        }
        true
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn handle_test_instance_block_editor_mouse_input(
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
        let Some(editor) = self.test_instance_block_editor.as_mut() else {
            return false;
        };
        editor.mouse_position = cursor_position.and_then(physical_position_floor_i32);
        if !matches!((button, state), (MouseButton::Left, ElementState::Pressed)) {
            return true;
        }
        let Some((mouse_x, mouse_y)) = editor.mouse_position else {
            return true;
        };
        let layout = hud_test_instance_block_editor_layout(surface_size);
        for (focus, rect) in [
            (TestInstanceBlockEditorFocus::Id, layout.id),
            (TestInstanceBlockEditorFocus::SizeX, layout.size[0]),
            (TestInstanceBlockEditorFocus::SizeY, layout.size[1]),
            (TestInstanceBlockEditorFocus::SizeZ, layout.size[2]),
        ] {
            if rect.contains(mouse_x, mouse_y) {
                editor.focus = focus;
                set_cursor_from_mouse(editor.field_mut(), rect, mouse_x, measurer);
                return true;
            }
        }
        if layout.rotation.contains(mouse_x, mouse_y) {
            editor.rotation = match editor.rotation {
                TestInstanceBlockRotation::None => TestInstanceBlockRotation::Clockwise90,
                TestInstanceBlockRotation::Clockwise90 => TestInstanceBlockRotation::Clockwise180,
                TestInstanceBlockRotation::Clockwise180 => {
                    TestInstanceBlockRotation::Counterclockwise90
                }
                TestInstanceBlockRotation::Counterclockwise90 => TestInstanceBlockRotation::None,
            };
            return true;
        }
        if layout.include_entities.contains(mouse_x, mouse_y) {
            editor.include_entities = !editor.include_entities;
            return true;
        }
        if layout.cancel.contains(mouse_x, mouse_y) {
            self.test_instance_block_editor = None;
            world.close_test_instance_block_editor();
            return true;
        }
        let action = if layout.reset.contains(mouse_x, mouse_y) {
            Some(TestInstanceBlockActionKind::Reset)
        } else if layout.save.contains(mouse_x, mouse_y) {
            editor
                .save_enabled()
                .then_some(TestInstanceBlockActionKind::Save)
        } else if layout.run.contains(mouse_x, mouse_y) {
            Some(TestInstanceBlockActionKind::Run)
        } else if layout.done.contains(mouse_x, mouse_y) {
            Some(TestInstanceBlockActionKind::Set)
        } else {
            None
        };
        if let Some(action) = action {
            self.send_test_instance_block_action(counters, world, net_commands, action);
        }
        true
    }

    fn queue_test_instance_block_query(
        &mut self,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
    ) {
        let Some(editor) = self.test_instance_block_editor.as_mut() else {
            return;
        };
        editor.local_invalid_id = editor.normalized_test().is_none();
        editor.response_status = None;
        let query = editor.action(TestInstanceBlockActionKind::Query);
        queue_test_instance_block_action_command(counters, net_commands, query);
    }

    fn send_test_instance_block_action(
        &mut self,
        counters: &mut NetCounters,
        world: &mut WorldStore,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        action: TestInstanceBlockActionKind,
    ) {
        let Some(packet) = self
            .test_instance_block_editor
            .as_ref()
            .map(|editor| editor.action(action))
        else {
            return;
        };
        queue_test_instance_block_action_command(counters, net_commands, packet);
        self.test_instance_block_editor = None;
        world.close_test_instance_block_editor();
    }
}

fn parse_size(text: &str) -> i32 {
    text.parse::<i32>().unwrap_or(1).clamp(1, 48)
}

fn truncate_utf16(text: String, max_utf16_units: usize) -> String {
    let mut units = 0usize;
    text.chars()
        .take_while(|ch| {
            let next = units.saturating_add(ch.len_utf16());
            let keep = next <= max_utf16_units;
            if keep {
                units = next;
            }
            keep
        })
        .collect()
}

fn delete_selection(field: &mut TestInstanceBlockEditorTextField) -> bool {
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

fn delete_before(field: &mut TestInstanceBlockEditorTextField, control_down: bool) -> bool {
    if field.cursor == 0 {
        return false;
    }
    let before = field.text.clone();
    if control_down {
        text_edit::remove_word_before_cursor(&mut field.text, &mut field.cursor);
    } else {
        let start = text_edit::byte_index(&field.text, field.cursor - 1);
        let end = text_edit::byte_index(&field.text, field.cursor);
        field.text.replace_range(start..end, "");
        field.cursor -= 1;
    }
    field.selection = field.cursor;
    field.text != before
}

fn delete_at(field: &mut TestInstanceBlockEditorTextField, control_down: bool) -> bool {
    if field.cursor >= text_edit::char_len(&field.text) {
        return false;
    }
    let before = field.text.clone();
    if control_down {
        text_edit::remove_word_at_cursor(&mut field.text, field.cursor);
    } else {
        let start = text_edit::byte_index(&field.text, field.cursor);
        let end = text_edit::byte_index(&field.text, field.cursor + 1);
        field.text.replace_range(start..end, "");
    }
    field.selection = field.cursor;
    field.text != before
}

fn insert_text(field: &mut TestInstanceBlockEditorTextField, text: &str) -> bool {
    let before = field.text.clone();
    delete_selection(field);
    let mut insert = String::new();
    let mut units = field.text.encode_utf16().count();
    for ch in text.chars().filter(|ch| is_edit_box_char(*ch)) {
        let next = units.saturating_add(ch.len_utf16());
        if next > field.max_utf16_units {
            break;
        }
        insert.push(ch);
        units = next;
    }
    if !insert.is_empty() {
        let byte = text_edit::byte_index(&field.text, field.cursor);
        field.text.insert_str(byte, &insert);
        field.cursor += text_edit::char_len(&insert);
        field.selection = field.cursor;
    }
    field.text != before
}

fn is_edit_box_char(ch: char) -> bool {
    ch >= ' ' && ch != '\u{7f}' && ch != '\u{a7}'
}

fn move_cursor(field: &mut TestInstanceBlockEditorTextField, cursor: usize, selecting: bool) {
    field.cursor = cursor.min(text_edit::char_len(&field.text));
    if !selecting {
        field.selection = field.cursor;
    }
}

fn set_cursor_from_mouse(
    field: &mut TestInstanceBlockEditorTextField,
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
