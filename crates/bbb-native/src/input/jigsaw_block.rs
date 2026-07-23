use bbb_control::NetCounters;
use bbb_net::NetCommand;
use bbb_protocol::normalize_resource_location;
use bbb_protocol::packets::{
    JigsawGenerate, JigsawJoint, SetJigsawBlock, MAX_JIGSAW_FINAL_STATE_UTF16_UNITS,
    MAX_JIGSAW_RESOURCE_UTF16_UNITS,
};
use bbb_renderer::{hud_jigsaw_block_editor_layout, HudCommandBlockEditorRect};
use bbb_world::{BlockPos, WorldStore};
use tokio::sync::mpsc;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton},
    keyboard::KeyCode,
};

use super::{
    physical_position_floor_i32, protocol_block_pos_from_world, queue_jigsaw_generate_command,
    queue_set_jigsaw_block_command, release_active_input, text_edit, ClientInputState,
    CommandSuggestionTextMeasurer,
};

const PRIORITY_FIELD_UTF16_UNITS: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum JigsawBlockEditorFocus {
    Pool,
    Name,
    Target,
    FinalState,
    SelectionPriority,
    PlacementPriority,
}

impl JigsawBlockEditorFocus {
    fn index(self) -> usize {
        match self {
            Self::Pool => 0,
            Self::Name => 1,
            Self::Target => 2,
            Self::FinalState => 3,
            Self::SelectionPriority => 4,
            Self::PlacementPriority => 5,
        }
    }

    fn from_index(index: usize) -> Self {
        match index % 6 {
            0 => Self::Pool,
            1 => Self::Name,
            2 => Self::Target,
            3 => Self::FinalState,
            4 => Self::SelectionPriority,
            _ => Self::PlacementPriority,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct JigsawBlockEditorTextField {
    pub(super) text: String,
    pub(super) cursor: usize,
    pub(super) selection: usize,
    pub(super) max_utf16_units: usize,
}

impl JigsawBlockEditorTextField {
    fn new(text: String, max_utf16_units: usize) -> Self {
        let text = truncate_utf16(text, max_utf16_units);
        let cursor = text_edit::char_len(&text);
        Self {
            text,
            cursor,
            selection: cursor,
            max_utf16_units,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct JigsawBlockEditorInputState {
    pub(super) open_count: usize,
    pub(super) authoritative_revision: usize,
    pub(super) pos: BlockPos,
    pub(super) fields: [JigsawBlockEditorTextField; 6],
    pub(super) focus: JigsawBlockEditorFocus,
    pub(super) joint: JigsawJoint,
    pub(super) joint_editable: bool,
    pub(super) slider_value_bits: u64,
    pub(super) levels: i32,
    pub(super) keep_jigsaws: bool,
    pub(super) dragging_levels: bool,
    pub(super) mouse_position: Option<(i32, i32)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct JigsawBlockEditorHudTextField {
    pub(crate) text: String,
    pub(crate) cursor: usize,
    pub(crate) selection: usize,
    pub(crate) max_utf16_units: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct JigsawBlockEditorHudState {
    pub(crate) fields: [JigsawBlockEditorHudTextField; 6],
    pub(crate) focus_index: usize,
    pub(crate) joint: JigsawJoint,
    pub(crate) joint_editable: bool,
    pub(crate) slider_value_bits: u64,
    pub(crate) levels: i32,
    pub(crate) keep_jigsaws: bool,
    pub(crate) dragging_levels: bool,
    pub(crate) mouse_position: Option<(i32, i32)>,
    pub(crate) controls_enabled: bool,
}

impl JigsawBlockEditorInputState {
    fn field(&self) -> &JigsawBlockEditorTextField {
        &self.fields[self.focus.index()]
    }

    fn field_mut(&mut self) -> &mut JigsawBlockEditorTextField {
        &mut self.fields[self.focus.index()]
    }

    fn normalized_resources(&self) -> Option<[String; 3]> {
        Some([
            normalize_resource_location(self.fields[1].text.clone()).ok()?,
            normalize_resource_location(self.fields[2].text.clone()).ok()?,
            normalize_resource_location(self.fields[0].text.clone()).ok()?,
        ])
    }

    fn controls_enabled(&self) -> bool {
        self.normalized_resources().is_some()
    }

    fn set_packet(&self) -> Option<SetJigsawBlock> {
        let [name, target, pool] = self.normalized_resources()?;
        Some(SetJigsawBlock {
            pos: protocol_block_pos_from_world(self.pos),
            name,
            target,
            pool,
            final_state: self.fields[3].text.clone(),
            joint: self.joint,
            selection_priority: parse_priority(&self.fields[4].text),
            placement_priority: parse_priority(&self.fields[5].text),
        })
    }

    fn generate_packet(&self) -> JigsawGenerate {
        JigsawGenerate {
            pos: protocol_block_pos_from_world(self.pos),
            levels: self.levels,
            keep_jigsaws: self.keep_jigsaws,
        }
    }

    fn update_slider_from_mouse(&mut self, mouse_x: i32, rect: HudCommandBlockEditorRect) {
        let usable = rect.width.saturating_sub(8).max(1);
        let value = (f64::from(mouse_x.saturating_sub(rect.x.saturating_add(4)))
            / f64::from(usable))
        .clamp(0.0, 1.0);
        self.slider_value_bits = value.to_bits();
        self.levels = (value * 20.0).floor() as i32;
    }
}

impl ClientInputState {
    pub(crate) fn jigsaw_block_editor_is_active(&self) -> bool {
        self.jigsaw_block_editor.is_some()
    }

    pub(crate) fn jigsaw_block_editor_hud_state(&self) -> Option<JigsawBlockEditorHudState> {
        let editor = self.jigsaw_block_editor.as_ref()?;
        Some(JigsawBlockEditorHudState {
            fields: editor
                .fields
                .clone()
                .map(|field| JigsawBlockEditorHudTextField {
                    text: field.text,
                    cursor: field.cursor,
                    selection: field.selection,
                    max_utf16_units: field.max_utf16_units,
                }),
            focus_index: editor.focus.index(),
            joint: editor.joint,
            joint_editable: editor.joint_editable,
            slider_value_bits: editor.slider_value_bits,
            levels: editor.levels,
            keep_jigsaws: editor.keep_jigsaws,
            dragging_levels: editor.dragging_levels,
            mouse_position: editor.mouse_position,
            controls_enabled: editor.controls_enabled(),
        })
    }

    pub(crate) fn sync_jigsaw_block_editor(
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
            || world.social_interactions_screen().is_some()
            || world.game_rules_screen().is_some()
            || self.sign_editor_is_active_or_pending(world)
            || self.debug_options_screen_is_open()
            || self.debug_pause_screen_is_open()
            || self.command_block_editor_is_active()
            || self.minecart_command_block_editor_is_active()
            || self.structure_block_editor_is_active()
            || self.test_block_editor_is_active()
            || self.test_instance_block_editor_is_active();
        if replaced {
            world.close_jigsaw_block_editor();
        }
        let Some(authoritative) = world.jigsaw_block_editor().cloned() else {
            self.jigsaw_block_editor = None;
            return;
        };
        let open_count = world.jigsaw_block_editor_open_count();
        if self
            .jigsaw_block_editor
            .as_ref()
            .is_some_and(|editor| editor.open_count == open_count)
        {
            if let Some(editor) = &mut self.jigsaw_block_editor {
                editor.authoritative_revision = authoritative.authoritative_revision;
                editor.joint_editable = authoritative.joint_editable;
            }
            return;
        }

        release_active_input(self, world, counters, net_commands);
        self.chat_entry = None;
        self.chat_scroll_lines = 0;
        self.structure_block_editor = None;
        self.test_block_editor = None;
        self.test_instance_block_editor = None;
        world.close_command_block_editor();
        world.close_structure_block_editor();
        world.close_test_block_editor();
        world.close_test_instance_block_editor();
        let data = authoritative.data;
        self.jigsaw_block_editor = Some(JigsawBlockEditorInputState {
            open_count,
            authoritative_revision: authoritative.authoritative_revision,
            pos: authoritative.pos,
            fields: [
                JigsawBlockEditorTextField::new(data.pool, MAX_JIGSAW_RESOURCE_UTF16_UNITS),
                JigsawBlockEditorTextField::new(data.name, MAX_JIGSAW_RESOURCE_UTF16_UNITS),
                JigsawBlockEditorTextField::new(data.target, MAX_JIGSAW_RESOURCE_UTF16_UNITS),
                JigsawBlockEditorTextField::new(
                    data.final_state,
                    MAX_JIGSAW_FINAL_STATE_UTF16_UNITS,
                ),
                JigsawBlockEditorTextField::new(
                    data.selection_priority.to_string(),
                    PRIORITY_FIELD_UTF16_UNITS,
                ),
                JigsawBlockEditorTextField::new(
                    data.placement_priority.to_string(),
                    PRIORITY_FIELD_UTF16_UNITS,
                ),
            ],
            focus: JigsawBlockEditorFocus::Pool,
            joint: data.joint,
            joint_editable: authoritative.joint_editable,
            slider_value_bits: 0.0_f64.to_bits(),
            levels: 0,
            keep_jigsaws: true,
            dragging_levels: false,
            mouse_position: None,
        });
    }

    pub(crate) fn handle_jigsaw_block_editor_cursor_moved(
        &mut self,
        cursor_position: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
    ) -> bool {
        let Some(editor) = self.jigsaw_block_editor.as_mut() else {
            return false;
        };
        editor.mouse_position = cursor_position.and_then(physical_position_floor_i32);
        if editor.dragging_levels {
            if let Some((mouse_x, _)) = editor.mouse_position {
                editor.update_slider_from_mouse(
                    mouse_x,
                    hud_jigsaw_block_editor_layout(surface_size).levels,
                );
            }
        }
        true
    }

    pub(crate) fn handle_jigsaw_block_editor_key(
        &mut self,
        counters: &mut NetCounters,
        world: &mut WorldStore,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        code: KeyCode,
        pressed: bool,
    ) -> bool {
        if world.dialog_screen_is_open() || self.jigsaw_block_editor.is_none() {
            return false;
        }
        if !pressed {
            return true;
        }
        if code == KeyCode::Escape {
            self.jigsaw_block_editor = None;
            world.close_jigsaw_block_editor();
            return true;
        }
        if matches!(code, KeyCode::Enter | KeyCode::NumpadEnter) {
            self.submit_jigsaw_block(counters, world, net_commands, false);
            return true;
        }
        let control_down = self.control_down();
        let shift_down = self.shift_down();
        let editor = self
            .jigsaw_block_editor
            .as_mut()
            .expect("checked jigsaw editor");
        match code {
            KeyCode::Tab => {
                let index = editor.focus.index();
                editor.focus = JigsawBlockEditorFocus::from_index(if shift_down {
                    index.wrapping_add(5)
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
                let field = editor.field_mut();
                if !delete_selection(field) {
                    delete_before(field, control_down);
                }
            }
            KeyCode::Delete => {
                let field = editor.field_mut();
                if !delete_selection(field) {
                    delete_at(field, control_down);
                }
            }
            _ => {}
        }
        true
    }

    pub(crate) fn handle_jigsaw_block_editor_text(
        &mut self,
        world: &WorldStore,
        text: &str,
    ) -> bool {
        if world.dialog_screen_is_open() {
            return false;
        }
        let Some(editor) = self.jigsaw_block_editor.as_mut() else {
            return false;
        };
        insert_text(editor.field_mut(), text);
        true
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn handle_jigsaw_block_editor_mouse_input(
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
        let Some(editor) = self.jigsaw_block_editor.as_mut() else {
            return false;
        };
        editor.mouse_position = cursor_position.and_then(physical_position_floor_i32);
        if matches!((button, state), (MouseButton::Left, ElementState::Released)) {
            editor.dragging_levels = false;
            return true;
        }
        if !matches!((button, state), (MouseButton::Left, ElementState::Pressed)) {
            return true;
        }
        let Some((mouse_x, mouse_y)) = editor.mouse_position else {
            return true;
        };
        let layout = hud_jigsaw_block_editor_layout(surface_size);
        for (index, rect) in layout.fields.into_iter().enumerate() {
            if rect.contains(mouse_x, mouse_y) {
                editor.focus = JigsawBlockEditorFocus::from_index(index);
                set_cursor_from_mouse(editor.field_mut(), rect, mouse_x, measurer);
                return true;
            }
        }
        if layout.levels.contains(mouse_x, mouse_y) {
            editor.dragging_levels = true;
            editor.update_slider_from_mouse(mouse_x, layout.levels);
            return true;
        }
        if layout.keep_jigsaws.contains(mouse_x, mouse_y) {
            editor.keep_jigsaws = !editor.keep_jigsaws;
            return true;
        }
        if editor.joint_editable && layout.joint.contains(mouse_x, mouse_y) {
            editor.joint = match editor.joint {
                JigsawJoint::Rollable => JigsawJoint::Aligned,
                JigsawJoint::Aligned => JigsawJoint::Rollable,
            };
            return true;
        }
        if layout.cancel.contains(mouse_x, mouse_y) {
            self.jigsaw_block_editor = None;
            world.close_jigsaw_block_editor();
            return true;
        }
        let generate = layout.generate.contains(mouse_x, mouse_y);
        if layout.done.contains(mouse_x, mouse_y) || generate {
            self.submit_jigsaw_block(counters, world, net_commands, generate);
        }
        true
    }

    fn submit_jigsaw_block(
        &mut self,
        counters: &mut NetCounters,
        world: &mut WorldStore,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        generate: bool,
    ) {
        let Some(editor) = self.jigsaw_block_editor.as_ref() else {
            return;
        };
        let Some(set_packet) = editor.set_packet() else {
            return;
        };
        let generate_packet = editor.generate_packet();
        queue_set_jigsaw_block_command(counters, net_commands, set_packet);
        self.jigsaw_block_editor = None;
        world.close_jigsaw_block_editor();
        if generate {
            queue_jigsaw_generate_command(counters, net_commands, generate_packet);
        }
    }
}

fn parse_priority(text: &str) -> i32 {
    text.parse::<i32>().unwrap_or(0)
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

fn delete_selection(field: &mut JigsawBlockEditorTextField) -> bool {
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

fn delete_before(field: &mut JigsawBlockEditorTextField, control_down: bool) -> bool {
    if field.cursor == 0 {
        return false;
    }
    let start = if control_down {
        text_edit::word_position(&field.text, field.cursor, -1)
    } else {
        field.cursor - 1
    };
    let start_byte = text_edit::byte_index(&field.text, start);
    let end_byte = text_edit::byte_index(&field.text, field.cursor);
    field.text.replace_range(start_byte..end_byte, "");
    field.cursor = start;
    field.selection = start;
    true
}

fn delete_at(field: &mut JigsawBlockEditorTextField, control_down: bool) -> bool {
    let length = text_edit::char_len(&field.text);
    if field.cursor >= length {
        return false;
    }
    let end = if control_down {
        text_edit::word_position(&field.text, field.cursor, 1)
    } else {
        field.cursor + 1
    };
    let start_byte = text_edit::byte_index(&field.text, field.cursor);
    let end_byte = text_edit::byte_index(&field.text, end);
    field.text.replace_range(start_byte..end_byte, "");
    field.selection = field.cursor;
    true
}

fn insert_text(field: &mut JigsawBlockEditorTextField, text: &str) -> bool {
    let start = field.cursor.min(field.selection);
    let end = field.cursor.max(field.selection);
    let start_byte = text_edit::byte_index(&field.text, start);
    let end_byte = text_edit::byte_index(&field.text, end);
    let mut candidate = field.text.clone();
    candidate.replace_range(start_byte..end_byte, "");
    let available = field
        .max_utf16_units
        .saturating_sub(candidate.encode_utf16().count());
    let mut units = 0usize;
    let inserted: String = text
        .chars()
        .filter(|ch| !ch.is_control())
        .take_while(|ch| {
            let next = units.saturating_add(ch.len_utf16());
            let keep = next <= available;
            if keep {
                units = next;
            }
            keep
        })
        .collect();
    if inserted.is_empty() && start == end {
        return false;
    }
    let inserted_chars = text_edit::char_len(&inserted);
    field.text.replace_range(start_byte..end_byte, &inserted);
    field.cursor = start.saturating_add(inserted_chars);
    field.selection = field.cursor;
    true
}

fn move_cursor(field: &mut JigsawBlockEditorTextField, cursor: usize, selecting: bool) {
    field.cursor = cursor.min(text_edit::char_len(&field.text));
    if !selecting {
        field.selection = field.cursor;
    }
}

fn set_cursor_from_mouse(
    field: &mut JigsawBlockEditorTextField,
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
