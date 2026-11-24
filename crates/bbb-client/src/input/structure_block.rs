use crate::control::NetCounters;
use bbb_client_net::NetCommand;
use bbb_client_render::hud_structure_block_editor_layout;
use bbb_client_world::{BlockPos, StructureBlockEditorState, WorldStore};
use bbb_protocol::packets::{
    BlockPos as ProtocolBlockPos, SetStructureBlock, StructureBlockMirror, StructureBlockMode,
    StructureBlockRotation, StructureBlockUpdateKind, Vec3i, MAX_STRUCTURE_BLOCK_OFFSET,
    MAX_STRUCTURE_BLOCK_SIZE, MAX_STRUCTURE_BLOCK_TEXT_UTF16_UNITS,
};
use tokio::sync::mpsc;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton},
    keyboard::KeyCode,
};

use super::{
    physical_position_floor_i32, queue_set_structure_block_command, screen::Screen,
    text_field::TextField, ClientInputState, CommandSuggestionTextMeasurer, DebugClipboard,
};

const NUMERIC_FIELD_UTF16_UNITS: usize = 15;
const SEED_FIELD_UTF16_UNITS: usize = 31;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StructureBlockEditorFocus {
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
    pub(crate) fn index(self) -> usize {
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
pub(crate) struct StructureBlockEditorInputState {
    pub(super) pos: BlockPos,
    pub(super) fields: [TextField; 10],
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
pub(crate) struct StructureBlockEditorHudState<'a> {
    pub(crate) fields: &'a [TextField; 10],
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
    pub(crate) fn focus_index(&self) -> usize {
        self.focus.index()
    }

    fn field_mut(&mut self) -> &mut TextField {
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
            pos: self.pos,
            update,
            mode: self.mode,
            name: self.fields[0].text(),
            offset: ProtocolBlockPos {
                x: parse_i32(&self.fields[1].text())
                    .clamp(-MAX_STRUCTURE_BLOCK_OFFSET, MAX_STRUCTURE_BLOCK_OFFSET),
                y: parse_i32(&self.fields[2].text())
                    .clamp(-MAX_STRUCTURE_BLOCK_OFFSET, MAX_STRUCTURE_BLOCK_OFFSET),
                z: parse_i32(&self.fields[3].text())
                    .clamp(-MAX_STRUCTURE_BLOCK_OFFSET, MAX_STRUCTURE_BLOCK_OFFSET),
            },
            size: Vec3i {
                x: parse_i32(&self.fields[4].text()).clamp(0, MAX_STRUCTURE_BLOCK_SIZE),
                y: parse_i32(&self.fields[5].text()).clamp(0, MAX_STRUCTURE_BLOCK_SIZE),
                z: parse_i32(&self.fields[6].text()).clamp(0, MAX_STRUCTURE_BLOCK_SIZE),
            },
            mirror: self.mirror,
            rotation: self.rotation,
            data: self.fields[9].value().to_vec().into(),
            ignore_entities: !self.include_entities,
            strict: self.strict,
            show_air: self.show_air,
            show_bounding_box: self.show_bounding_box,
            integrity: parse_integrity(&self.fields[7].text()),
            seed: self.fields[8].text().parse::<i64>().unwrap_or(0),
        }
    }
}

impl ClientInputState {
    pub(crate) fn structure_block_editor_is_active(&self) -> bool {
        self.screen.structure_block().is_some()
    }

    pub(crate) fn structure_block_editor_hud_state(
        &self,
    ) -> Option<StructureBlockEditorHudState<'_>> {
        let editor = self.screen.structure_block()?;
        Some(StructureBlockEditorHudState {
            fields: &editor.fields,
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

    pub(crate) fn open_structure_block_editor(
        &mut self,
        world: &mut WorldStore,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        authoritative: StructureBlockEditorState,
    ) {
        let data = authoritative.data;
        let integrity = data.integrity();
        let mode = data.mode;
        let mirror = data.mirror;
        let rotation = data.rotation;
        let include_entities = !data.ignore_entities;
        let strict = data.strict;
        let show_air = data.show_air;
        let show_bounding_box = data.show_bounding_box;
        let editor = StructureBlockEditorInputState {
            pos: authoritative.pos,
            fields: [
                TextField::with_text(&data.name, MAX_STRUCTURE_BLOCK_TEXT_UTF16_UNITS),
                TextField::with_text(&data.offset.x.to_string(), NUMERIC_FIELD_UTF16_UNITS),
                TextField::with_text(&data.offset.y.to_string(), NUMERIC_FIELD_UTF16_UNITS),
                TextField::with_text(&data.offset.z.to_string(), NUMERIC_FIELD_UTF16_UNITS),
                TextField::with_text(&data.size.x.to_string(), NUMERIC_FIELD_UTF16_UNITS),
                TextField::with_text(&data.size.y.to_string(), NUMERIC_FIELD_UTF16_UNITS),
                TextField::with_text(&data.size.z.to_string(), NUMERIC_FIELD_UTF16_UNITS),
                TextField::with_text(&format_integrity(integrity), NUMERIC_FIELD_UTF16_UNITS),
                TextField::with_text(&data.seed.to_string(), SEED_FIELD_UTF16_UNITS),
                TextField::with_value(data.metadata.units(), MAX_STRUCTURE_BLOCK_TEXT_UTF16_UNITS),
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
        };
        self.open_editor_screen(
            Screen::StructureBlock(editor),
            world,
            counters,
            net_commands,
        );
    }

    pub(crate) fn handle_structure_block_editor_cursor_moved(
        &mut self,
        cursor_position: Option<PhysicalPosition<f64>>,
    ) -> bool {
        let Some(editor) = self.screen.structure_block_mut() else {
            return false;
        };
        editor.mouse_position = cursor_position.and_then(physical_position_floor_i32);
        true
    }

    pub(crate) fn handle_structure_block_editor_key(
        &mut self,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        code: KeyCode,
        pressed: bool,
        clipboard: &mut Option<&mut dyn DebugClipboard>,
    ) -> bool {
        if self.dialog_screen_is_open() || self.screen.structure_block().is_none() {
            return false;
        }
        if !pressed {
            return true;
        }
        if code == KeyCode::Escape {
            self.screen.close();
            return true;
        }
        if matches!(code, KeyCode::Enter | KeyCode::NumpadEnter) {
            self.submit_structure_block(
                counters,
                net_commands,
                StructureBlockUpdateKind::UpdateData,
            );
            return true;
        }
        let modifiers = self.text_field_modifiers();
        let editor = self.screen.structure_block_mut().expect("checked editor");
        if code == KeyCode::Tab {
            let focuses = editor.visible_focuses();
            let index = focuses
                .iter()
                .position(|focus| *focus == editor.focus)
                .unwrap_or(0);
            editor.focus = focuses[if modifiers.shift {
                index.checked_sub(1).unwrap_or(focuses.len() - 1)
            } else {
                (index + 1) % focuses.len()
            }];
            editor.field_mut().focus();
        } else {
            editor.field_mut().key(code, modifiers, clipboard, None);
        }
        true
    }

    pub(crate) fn handle_structure_block_editor_text(&mut self, text: &str) -> bool {
        if self.dialog_screen_is_open() {
            return false;
        }
        let Some(editor) = self.screen.structure_block_mut() else {
            return false;
        };
        if editor.focus == StructureBlockEditorFocus::Name {
            editor
                .field_mut()
                .type_text_filtered(text, valid_name_character, None);
        } else {
            editor.field_mut().type_text(text, None);
        }
        true
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn handle_structure_block_editor_mouse_input(
        &mut self,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        button: MouseButton,
        state: ElementState,
        cursor_position: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
        measurer: &impl CommandSuggestionTextMeasurer,
    ) -> bool {
        let alt_down = self.alt_down();
        let shift_down = self.shift_down();
        let Some(editor) = self.screen.structure_block_mut() else {
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
                if editor.focus != *focus {
                    editor.focus = *focus;
                    editor.field_mut().focus();
                }
                editor.field_mut().click_bordered(
                    rect,
                    mouse_x,
                    measurer.command_suggestion_glyphs(),
                    shift_down,
                );
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
            self.screen.close();
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
            self.submit_structure_block(counters, net_commands, update);
        }
        true
    }

    fn submit_structure_block(
        &mut self,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        update: StructureBlockUpdateKind,
    ) {
        let Some(packet) = self
            .screen
            .structure_block()
            .map(|editor| editor.packet(update))
        else {
            return;
        };
        queue_set_structure_block_command(counters, net_commands, packet);
        self.screen.close();
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

/// `Screen.isValidCharacterForName`.
fn valid_name_character(field: &TextField, new_char: char) -> bool {
    let value = field.value();
    let colon = value.iter().position(|&unit| unit == u16::from(b':'));
    let slash = value.iter().position(|&unit| unit == u16::from(b'/'));
    let cursor = field.cursor();
    match new_char {
        ':' => slash.is_none_or(|slash| cursor <= slash) && colon.is_none(),
        '/' => colon.is_none_or(|colon| cursor > colon),
        '_' | '-' | '.' | 'a'..='z' | '0'..='9' => true,
        _ => false,
    }
}
