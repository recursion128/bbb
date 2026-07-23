use bbb_control::NetCounters;
use bbb_net::NetCommand;
use bbb_protocol::packets::{
    BlockPos as ProtocolBlockPos, SetStructureBlock, StructureBlockMirror, StructureBlockMode,
    StructureBlockRotation, StructureBlockUpdateKind, Vec3i, MAX_STRUCTURE_BLOCK_OFFSET,
    MAX_STRUCTURE_BLOCK_SIZE, MAX_STRUCTURE_BLOCK_TEXT_UTF16_UNITS,
};
use bbb_renderer::{hud_structure_block_editor_layout, HudCommandBlockEditorRect};
use bbb_world::{BlockPos, WorldStore};
use tokio::sync::mpsc;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton},
    keyboard::KeyCode,
};

use super::{
    physical_position_floor_i32, protocol_block_pos_from_world, queue_set_structure_block_command,
    release_active_input, text_edit, ClientInputState, CommandSuggestionTextMeasurer,
};

const NUMERIC_FIELD_UTF16_UNITS: usize = 15;
const SEED_FIELD_UTF16_UNITS: usize = 31;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StructureBlockEditorFocus {
    Name,
    OffsetX,
    OffsetY,
    OffsetZ,
    SizeX,
    SizeY,
    SizeZ,
    Integrity,
    Seed,
    Data,
}

impl StructureBlockEditorFocus {
    fn index(self) -> usize {
        match self {
            Self::Name => 0,
            Self::OffsetX => 1,
            Self::OffsetY => 2,
            Self::OffsetZ => 3,
            Self::SizeX => 4,
            Self::SizeY => 5,
            Self::SizeZ => 6,
            Self::Integrity => 7,
            Self::Seed => 8,
            Self::Data => 9,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StructureBlockEditorTextField {
    pub(super) text: String,
    pub(super) cursor: usize,
    pub(super) selection: usize,
    pub(super) max_utf16_units: usize,
}

impl StructureBlockEditorTextField {
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
pub(super) struct StructureBlockEditorInputState {
    pub(super) open_count: usize,
    pub(super) authoritative_revision: usize,
    pub(super) pos: BlockPos,
    pub(super) fields: [StructureBlockEditorTextField; 10],
    pub(super) focus: StructureBlockEditorFocus,
    pub(super) mode: StructureBlockMode,
    pub(super) mirror: StructureBlockMirror,
    pub(super) rotation: StructureBlockRotation,
    pub(super) include_entities: bool,
    pub(super) strict: bool,
    pub(super) show_air: bool,
    pub(super) show_bounding_box: bool,
    pub(super) mouse_position: Option<(i32, i32)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StructureBlockEditorHudTextField {
    pub(crate) text: String,
    pub(crate) cursor: usize,
    pub(crate) selection: usize,
    pub(crate) max_utf16_units: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StructureBlockEditorHudState {
    pub(crate) fields: [StructureBlockEditorHudTextField; 10],
    pub(crate) focus_index: usize,
    pub(crate) mode: StructureBlockMode,
    pub(crate) mirror: StructureBlockMirror,
    pub(crate) rotation: StructureBlockRotation,
    pub(crate) include_entities: bool,
    pub(crate) strict: bool,
    pub(crate) show_air: bool,
    pub(crate) show_bounding_box: bool,
    pub(crate) mouse_position: Option<(i32, i32)>,
}

impl StructureBlockEditorInputState {
    fn field(&self) -> &StructureBlockEditorTextField {
        &self.fields[self.focus.index()]
    }

    fn field_mut(&mut self) -> &mut StructureBlockEditorTextField {
        &mut self.fields[self.focus.index()]
    }

    fn visible_focuses(&self) -> &'static [StructureBlockEditorFocus] {
        use StructureBlockEditorFocus as Focus;
        match self.mode {
            StructureBlockMode::Save => &[
                Focus::Name,
                Focus::OffsetX,
                Focus::OffsetY,
                Focus::OffsetZ,
                Focus::SizeX,
                Focus::SizeY,
                Focus::SizeZ,
            ],
            StructureBlockMode::Load => &[
                Focus::Name,
                Focus::OffsetX,
                Focus::OffsetY,
                Focus::OffsetZ,
                Focus::Integrity,
                Focus::Seed,
            ],
            StructureBlockMode::Corner => &[Focus::Name],
            StructureBlockMode::Data => &[Focus::Data],
        }
    }

    fn ensure_visible_focus(&mut self) {
        if !self.visible_focuses().contains(&self.focus) {
            self.focus = self.visible_focuses()[0];
        }
    }

    fn packet(&self, update: StructureBlockUpdateKind) -> SetStructureBlock {
        SetStructureBlock {
            pos: protocol_block_pos_from_world(self.pos),
            update,
            mode: self.mode,
            name: self.fields[0].text.clone(),
            offset: ProtocolBlockPos {
                x: parse_i32(&self.fields[1].text)
                    .clamp(-MAX_STRUCTURE_BLOCK_OFFSET, MAX_STRUCTURE_BLOCK_OFFSET),
                y: parse_i32(&self.fields[2].text)
                    .clamp(-MAX_STRUCTURE_BLOCK_OFFSET, MAX_STRUCTURE_BLOCK_OFFSET),
                z: parse_i32(&self.fields[3].text)
                    .clamp(-MAX_STRUCTURE_BLOCK_OFFSET, MAX_STRUCTURE_BLOCK_OFFSET),
            },
            size: Vec3i {
                x: parse_i32(&self.fields[4].text).clamp(0, MAX_STRUCTURE_BLOCK_SIZE),
                y: parse_i32(&self.fields[5].text).clamp(0, MAX_STRUCTURE_BLOCK_SIZE),
                z: parse_i32(&self.fields[6].text).clamp(0, MAX_STRUCTURE_BLOCK_SIZE),
            },
            mirror: self.mirror,
            rotation: self.rotation,
            data: self.fields[9].text.clone(),
            ignore_entities: !self.include_entities,
            strict: self.strict,
            show_air: self.show_air,
            show_bounding_box: self.show_bounding_box,
            integrity: parse_integrity(&self.fields[7].text),
            seed: self.fields[8].text.parse::<i64>().unwrap_or(0),
        }
    }
}

impl ClientInputState {
    pub(crate) fn structure_block_editor_is_active(&self) -> bool {
        self.structure_block_editor.is_some()
    }

    pub(crate) fn structure_block_editor_hud_state(&self) -> Option<StructureBlockEditorHudState> {
        let editor = self.structure_block_editor.as_ref()?;
        Some(StructureBlockEditorHudState {
            fields: editor
                .fields
                .clone()
                .map(|field| StructureBlockEditorHudTextField {
                    text: field.text,
                    cursor: field.cursor,
                    selection: field.selection,
                    max_utf16_units: field.max_utf16_units,
                }),
            focus_index: editor.focus.index(),
            mode: editor.mode,
            mirror: editor.mirror,
            rotation: editor.rotation,
            include_entities: editor.include_entities,
            strict: editor.strict,
            show_air: editor.show_air,
            show_bounding_box: editor.show_bounding_box,
            mouse_position: editor.mouse_position,
        })
    }

    pub(crate) fn sync_structure_block_editor(
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
            || self.jigsaw_block_editor_is_active()
            || self.test_block_editor_is_active()
            || self.test_instance_block_editor_is_active();
        if replaced {
            world.close_structure_block_editor();
        }
        let Some(authoritative) = world.structure_block_editor().cloned() else {
            self.structure_block_editor = None;
            return;
        };
        let open_count = world.structure_block_editor_open_count();
        if self
            .structure_block_editor
            .as_ref()
            .is_some_and(|editor| editor.open_count == open_count)
        {
            if let Some(editor) = &mut self.structure_block_editor {
                editor.authoritative_revision = authoritative.authoritative_revision;
            }
            return;
        }

        release_active_input(self, world, counters, net_commands);
        self.chat_entry = None;
        self.chat_scroll_lines = 0;
        self.jigsaw_block_editor = None;
        self.test_block_editor = None;
        self.test_instance_block_editor = None;
        world.close_command_block_editor();
        world.close_jigsaw_block_editor();
        world.close_test_block_editor();
        world.close_test_instance_block_editor();
        let data = authoritative.data;
        let integrity = data.integrity();
        let mode = data.mode;
        let mirror = data.mirror;
        let rotation = data.rotation;
        let include_entities = !data.ignore_entities;
        let strict = data.strict;
        let show_air = data.show_air;
        let show_bounding_box = data.show_bounding_box;
        self.structure_block_editor = Some(StructureBlockEditorInputState {
            open_count,
            authoritative_revision: authoritative.authoritative_revision,
            pos: authoritative.pos,
            fields: [
                StructureBlockEditorTextField::new(data.name, MAX_STRUCTURE_BLOCK_TEXT_UTF16_UNITS),
                StructureBlockEditorTextField::new(
                    data.offset.x.to_string(),
                    NUMERIC_FIELD_UTF16_UNITS,
                ),
                StructureBlockEditorTextField::new(
                    data.offset.y.to_string(),
                    NUMERIC_FIELD_UTF16_UNITS,
                ),
                StructureBlockEditorTextField::new(
                    data.offset.z.to_string(),
                    NUMERIC_FIELD_UTF16_UNITS,
                ),
                StructureBlockEditorTextField::new(
                    data.size.x.to_string(),
                    NUMERIC_FIELD_UTF16_UNITS,
                ),
                StructureBlockEditorTextField::new(
                    data.size.y.to_string(),
                    NUMERIC_FIELD_UTF16_UNITS,
                ),
                StructureBlockEditorTextField::new(
                    data.size.z.to_string(),
                    NUMERIC_FIELD_UTF16_UNITS,
                ),
                StructureBlockEditorTextField::new(
                    format_integrity(integrity),
                    NUMERIC_FIELD_UTF16_UNITS,
                ),
                StructureBlockEditorTextField::new(data.seed.to_string(), SEED_FIELD_UTF16_UNITS),
                StructureBlockEditorTextField::new(
                    data.metadata,
                    MAX_STRUCTURE_BLOCK_TEXT_UTF16_UNITS,
                ),
            ],
            focus: if mode == StructureBlockMode::Data {
                StructureBlockEditorFocus::Data
            } else {
                StructureBlockEditorFocus::Name
            },
            mode,
            mirror,
            rotation,
            include_entities,
            strict,
            show_air,
            show_bounding_box,
            mouse_position: None,
        });
    }

    pub(crate) fn handle_structure_block_editor_cursor_moved(
        &mut self,
        cursor_position: Option<PhysicalPosition<f64>>,
    ) -> bool {
        let Some(editor) = self.structure_block_editor.as_mut() else {
            return false;
        };
        editor.mouse_position = cursor_position.and_then(physical_position_floor_i32);
        true
    }

    pub(crate) fn handle_structure_block_editor_key(
        &mut self,
        counters: &mut NetCounters,
        world: &mut WorldStore,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        code: KeyCode,
        pressed: bool,
    ) -> bool {
        if world.dialog_screen_is_open() || self.structure_block_editor.is_none() {
            return false;
        }
        if !pressed {
            return true;
        }
        if code == KeyCode::Escape {
            self.structure_block_editor = None;
            world.close_structure_block_editor();
            return true;
        }
        if matches!(code, KeyCode::Enter | KeyCode::NumpadEnter) {
            self.submit_structure_block(
                counters,
                world,
                net_commands,
                StructureBlockUpdateKind::UpdateData,
            );
            return true;
        }
        let control_down = self.control_down();
        let shift_down = self.shift_down();
        let editor = self
            .structure_block_editor
            .as_mut()
            .expect("checked editor");
        match code {
            KeyCode::Tab => {
                let focuses = editor.visible_focuses();
                let index = focuses
                    .iter()
                    .position(|focus| *focus == editor.focus)
                    .unwrap_or(0);
                editor.focus = focuses[if shift_down {
                    index.checked_sub(1).unwrap_or(focuses.len() - 1)
                } else {
                    (index + 1) % focuses.len()
                }];
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
                let len = text_edit::char_len(&editor.field().text);
                move_cursor(editor.field_mut(), len, shift_down);
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

    pub(crate) fn handle_structure_block_editor_text(
        &mut self,
        world: &WorldStore,
        text: &str,
    ) -> bool {
        if world.dialog_screen_is_open() {
            return false;
        }
        let Some(editor) = self.structure_block_editor.as_mut() else {
            return false;
        };
        let name = editor.focus == StructureBlockEditorFocus::Name;
        insert_text(editor.field_mut(), text, name);
        true
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn handle_structure_block_editor_mouse_input(
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
        let alt_down = self.alt_down();
        let Some(editor) = self.structure_block_editor.as_mut() else {
            return false;
        };
        editor.mouse_position = cursor_position.and_then(physical_position_floor_i32);
        if !matches!((button, state), (MouseButton::Left, ElementState::Pressed)) {
            return true;
        }
        let Some((mouse_x, mouse_y)) = editor.mouse_position else {
            return true;
        };
        let layout = hud_structure_block_editor_layout(surface_size);
        for focus in editor.visible_focuses() {
            let rect = layout.fields[focus.index()];
            if rect.contains(mouse_x, mouse_y) {
                editor.focus = *focus;
                set_cursor_from_mouse(editor.field_mut(), rect, mouse_x, measurer);
                return true;
            }
        }
        if layout.mode.contains(mouse_x, mouse_y) {
            editor.mode = cycle_mode(editor.mode, alt_down);
            editor.ensure_visible_focus();
            return true;
        }
        if editor.mode == StructureBlockMode::Load && layout.mirror.contains(mouse_x, mouse_y) {
            editor.mirror = match editor.mirror {
                StructureBlockMirror::None => StructureBlockMirror::LeftRight,
                StructureBlockMirror::LeftRight => StructureBlockMirror::FrontBack,
                StructureBlockMirror::FrontBack => StructureBlockMirror::None,
            };
            return true;
        }
        if editor.mode == StructureBlockMode::Load {
            for (rect, rotation) in [
                (layout.rotations[0], StructureBlockRotation::None),
                (layout.rotations[1], StructureBlockRotation::Clockwise90),
                (layout.rotations[2], StructureBlockRotation::Clockwise180),
                (
                    layout.rotations[3],
                    StructureBlockRotation::Counterclockwise90,
                ),
            ] {
                if rect.contains(mouse_x, mouse_y) {
                    editor.rotation = rotation;
                    return true;
                }
            }
        }
        if matches!(
            editor.mode,
            StructureBlockMode::Save | StructureBlockMode::Load
        ) && layout.include_entities.contains(mouse_x, mouse_y)
        {
            editor.include_entities = !editor.include_entities;
            return true;
        }
        if editor.mode == StructureBlockMode::Load && layout.strict.contains(mouse_x, mouse_y) {
            editor.strict = !editor.strict;
            return true;
        }
        if editor.mode == StructureBlockMode::Save && layout.option.contains(mouse_x, mouse_y) {
            editor.show_air = !editor.show_air;
            return true;
        }
        if editor.mode == StructureBlockMode::Load && layout.option.contains(mouse_x, mouse_y) {
            editor.show_bounding_box = !editor.show_bounding_box;
            return true;
        }
        if layout.cancel.contains(mouse_x, mouse_y) {
            self.structure_block_editor = None;
            world.close_structure_block_editor();
            return true;
        }
        let update = if layout.done.contains(mouse_x, mouse_y) {
            Some(StructureBlockUpdateKind::UpdateData)
        } else if layout.action.contains(mouse_x, mouse_y) {
            match editor.mode {
                StructureBlockMode::Save => Some(StructureBlockUpdateKind::SaveArea),
                StructureBlockMode::Load => Some(StructureBlockUpdateKind::LoadArea),
                _ => None,
            }
        } else if editor.mode == StructureBlockMode::Save
            && layout.detect.contains(mouse_x, mouse_y)
        {
            Some(StructureBlockUpdateKind::ScanArea)
        } else {
            None
        };
        if let Some(update) = update {
            self.submit_structure_block(counters, world, net_commands, update);
        }
        true
    }

    fn submit_structure_block(
        &mut self,
        counters: &mut NetCounters,
        world: &mut WorldStore,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        update: StructureBlockUpdateKind,
    ) {
        let Some(packet) = self
            .structure_block_editor
            .as_ref()
            .map(|editor| editor.packet(update))
        else {
            return;
        };
        queue_set_structure_block_command(counters, net_commands, packet);
        self.structure_block_editor = None;
        world.close_structure_block_editor();
    }
}

fn cycle_mode(mode: StructureBlockMode, include_data: bool) -> StructureBlockMode {
    match (mode, include_data) {
        (StructureBlockMode::Save, _) => StructureBlockMode::Load,
        (StructureBlockMode::Load, _) => StructureBlockMode::Corner,
        (StructureBlockMode::Corner, true) => StructureBlockMode::Data,
        (StructureBlockMode::Corner, false) | (StructureBlockMode::Data, _) => {
            StructureBlockMode::Save
        }
    }
}

fn parse_i32(text: &str) -> i32 {
    text.parse::<i32>().unwrap_or(0)
}

fn parse_integrity(text: &str) -> f32 {
    text.parse::<f32>().unwrap_or(1.0).clamp(0.0, 1.0)
}

fn format_integrity(value: f32) -> String {
    let mut value = format!("{value:.4}");
    while value.contains('.') && value.ends_with('0') {
        value.pop();
    }
    if value.ends_with('.') {
        value.push('0');
    }
    value
}

fn valid_name_character(current: &str, new_char: char, cursor: usize) -> bool {
    let colon = current.chars().position(|ch| ch == ':');
    let slash = current.chars().position(|ch| ch == '/');
    match new_char {
        ':' => slash.is_none_or(|slash| cursor <= slash) && colon.is_none(),
        '/' => colon.is_none_or(|colon| cursor > colon),
        '_' | '-' | '.' | 'a'..='z' | '0'..='9' => true,
        _ => false,
    }
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

fn delete_selection(field: &mut StructureBlockEditorTextField) -> bool {
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

fn delete_before(field: &mut StructureBlockEditorTextField, control_down: bool) -> bool {
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

fn delete_at(field: &mut StructureBlockEditorTextField, control_down: bool) -> bool {
    let len = text_edit::char_len(&field.text);
    if field.cursor >= len {
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

fn insert_text(field: &mut StructureBlockEditorTextField, text: &str, filter_name: bool) -> bool {
    delete_selection(field);
    let mut changed = false;
    for ch in text.chars().filter(|ch| !ch.is_control()) {
        if filter_name && !valid_name_character(&field.text, ch, field.cursor) {
            continue;
        }
        let current_units = field.text.encode_utf16().count();
        if current_units.saturating_add(ch.len_utf16()) > field.max_utf16_units {
            break;
        }
        let byte = text_edit::byte_index(&field.text, field.cursor);
        field.text.insert(byte, ch);
        field.cursor += 1;
        field.selection = field.cursor;
        changed = true;
    }
    changed
}

fn move_cursor(field: &mut StructureBlockEditorTextField, cursor: usize, selecting: bool) {
    field.cursor = cursor.min(text_edit::char_len(&field.text));
    if !selecting {
        field.selection = field.cursor;
    }
}

fn set_cursor_from_mouse(
    field: &mut StructureBlockEditorTextField,
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
