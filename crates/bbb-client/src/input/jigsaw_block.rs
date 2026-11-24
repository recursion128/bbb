use crate::control::NetCounters;
use bbb_client_net::NetCommand;
use bbb_client_render::{hud_jigsaw_block_editor_layout, HudCommandBlockEditorRect};
use bbb_client_world::{BlockPos, JigsawBlockEditorState, WorldStore};
use bbb_protocol::normalize_resource_location;
use bbb_protocol::packets::{
    JigsawGenerate, JigsawJoint, SetJigsawBlock, MAX_JIGSAW_FINAL_STATE_UTF16_UNITS,
    MAX_JIGSAW_RESOURCE_UTF16_UNITS,
};
use tokio::sync::mpsc;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton},
    keyboard::KeyCode,
};

use super::{
    physical_position_floor_i32, queue_jigsaw_generate_command, queue_set_jigsaw_block_command,
    screen::Screen, text_field::TextField, ClientInputState, CommandSuggestionTextMeasurer,
    DebugClipboard,
};

const PRIORITY_FIELD_UTF16_UNITS: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum JigsawBlockEditorFocus {
    Pool,
    Name,
    Target,
    FinalState,
    SelectionPriority,
    PlacementPriority,
}

impl JigsawBlockEditorFocus {
    pub(crate) fn index(self) -> usize {
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
pub(crate) struct JigsawBlockEditorInputState {
    pub(super) pos: BlockPos,
    pub(super) fields: [TextField; 6],
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
pub(crate) struct JigsawBlockEditorHudState<'a> {
    pub(crate) fields: &'a [TextField; 6],
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
    pub(crate) fn focus_index(&self) -> usize {
        self.focus.index()
    }

    fn field_mut(&mut self) -> &mut TextField {
        &mut self.fields[self.focus.index()]
    }

    fn normalized_resources(&self) -> Option<[String; 3]> {
        Some([
            normalize_resource_location(self.fields[1].text()).ok()?,
            normalize_resource_location(self.fields[2].text()).ok()?,
            normalize_resource_location(self.fields[0].text()).ok()?,
        ])
    }

    fn controls_enabled(&self) -> bool {
        self.normalized_resources().is_some()
    }

    fn set_packet(&self) -> Option<SetJigsawBlock> {
        let [name, target, pool] = self.normalized_resources()?;
        Some(SetJigsawBlock {
            pos: self.pos,
            name,
            target,
            pool,
            final_state: self.fields[3].value().to_vec().into(),
            joint: self.joint,
            selection_priority: parse_priority(&self.fields[4].text()),
            placement_priority: parse_priority(&self.fields[5].text()),
        })
    }

    fn generate_packet(&self) -> JigsawGenerate {
        JigsawGenerate {
            pos: self.pos,
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
        self.screen.jigsaw_block().is_some()
    }

    pub(crate) fn jigsaw_block_editor_hud_state(&self) -> Option<JigsawBlockEditorHudState<'_>> {
        let editor = self.screen.jigsaw_block()?;
        Some(JigsawBlockEditorHudState {
            fields: &editor.fields,
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

    pub(crate) fn open_jigsaw_block_editor(
        &mut self,
        world: &mut WorldStore,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        authoritative: JigsawBlockEditorState,
    ) {
        let data = authoritative.data;
        let editor = JigsawBlockEditorInputState {
            pos: authoritative.pos,
            fields: [
                TextField::with_text(&data.pool, MAX_JIGSAW_RESOURCE_UTF16_UNITS),
                TextField::with_text(&data.name, MAX_JIGSAW_RESOURCE_UTF16_UNITS),
                TextField::with_text(&data.target, MAX_JIGSAW_RESOURCE_UTF16_UNITS),
                TextField::with_value(data.final_state.units(), MAX_JIGSAW_FINAL_STATE_UTF16_UNITS),
                TextField::with_text(
                    &data.selection_priority.to_string(),
                    PRIORITY_FIELD_UTF16_UNITS,
                ),
                TextField::with_text(
                    &data.placement_priority.to_string(),
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
        };
        self.open_editor_screen(Screen::JigsawBlock(editor), world, counters, net_commands);
    }

    pub(crate) fn handle_jigsaw_block_editor_cursor_moved(
        &mut self,
        cursor_position: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
    ) -> bool {
        let Some(editor) = self.screen.jigsaw_block_mut() else {
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
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        code: KeyCode,
        pressed: bool,
        clipboard: &mut Option<&mut dyn DebugClipboard>,
    ) -> bool {
        if self.dialog_screen_is_open() || self.screen.jigsaw_block().is_none() {
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
            self.submit_jigsaw_block(counters, net_commands, false);
            return true;
        }
        let modifiers = self.text_field_modifiers();
        let editor = self
            .screen
            .jigsaw_block_mut()
            .expect("checked jigsaw editor");
        if code == KeyCode::Tab {
            let index = editor.focus.index();
            editor.focus = JigsawBlockEditorFocus::from_index(if modifiers.shift {
                index.wrapping_add(5)
            } else {
                index + 1
            });
            editor.field_mut().focus();
        } else {
            editor.field_mut().key(code, modifiers, clipboard, None);
        }
        true
    }

    pub(crate) fn handle_jigsaw_block_editor_text(&mut self, text: &str) -> bool {
        if self.dialog_screen_is_open() {
            return false;
        }
        let Some(editor) = self.screen.jigsaw_block_mut() else {
            return false;
        };
        editor.field_mut().type_text(text, None);
        true
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn handle_jigsaw_block_editor_mouse_input(
        &mut self,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        button: MouseButton,
        state: ElementState,
        cursor_position: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
        measurer: &impl CommandSuggestionTextMeasurer,
    ) -> bool {
        let shift_down = self.shift_down();
        let Some(editor) = self.screen.jigsaw_block_mut() else {
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
                let focus = JigsawBlockEditorFocus::from_index(index);
                if editor.focus != focus {
                    editor.focus = focus;
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
            self.screen.close();
            return true;
        }
        let generate = layout.generate.contains(mouse_x, mouse_y);
        if layout.done.contains(mouse_x, mouse_y) || generate {
            self.submit_jigsaw_block(counters, net_commands, generate);
        }
        true
    }

    fn submit_jigsaw_block(
        &mut self,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        generate: bool,
    ) {
        let Some(editor) = self.screen.jigsaw_block() else {
            return;
        };
        let Some(set_packet) = editor.set_packet() else {
            return;
        };
        let generate_packet = editor.generate_packet();
        queue_set_jigsaw_block_command(counters, net_commands, set_packet);
        self.screen.close();
        if generate {
            queue_jigsaw_generate_command(counters, net_commands, generate_packet);
        }
    }
}

fn parse_priority(text: &str) -> i32 {
    text.parse::<i32>().unwrap_or(0)
}
