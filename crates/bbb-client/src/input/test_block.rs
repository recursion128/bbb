use crate::control::NetCounters;
use bbb_client_net::NetCommand;
use bbb_client_render::hud_test_block_editor_layout;
use bbb_client_world::{BlockPos, TestBlockEditorState, WorldStore};
use bbb_protocol::packets::{SetTestBlock, TestBlockMode, MAX_TEST_BLOCK_MESSAGE_UTF16_UNITS};
use tokio::sync::mpsc;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton},
    keyboard::KeyCode,
};

use super::{
    physical_position_floor_i32, queue_set_test_block_command, screen::Screen,
    text_field::TextField, ClientInputState, CommandSuggestionTextMeasurer, DebugClipboard,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TestBlockEditorInputState {
    pos: BlockPos,
    message: TextField,
    mode: TestBlockMode,
    mouse_position: Option<(i32, i32)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TestBlockEditorHudState<'a> {
    pub(crate) message: &'a TextField,
    pub(crate) mode: TestBlockMode,
    pub(crate) mouse_position: Option<(i32, i32)>,
}

impl TestBlockEditorInputState {
    pub(crate) fn message_is_editable(&self) -> bool {
        self.mode != TestBlockMode::Start
    }

    pub(super) fn message_mut(&mut self) -> &mut TextField {
        &mut self.message
    }

    fn packet(&self) -> SetTestBlock {
        SetTestBlock {
            pos: self.pos,
            mode: self.mode,
            message: self.message.value().to_vec().into(),
        }
    }
}

impl ClientInputState {
    pub(crate) fn test_block_editor_is_active(&self) -> bool {
        self.screen.test_block().is_some()
    }

    pub(crate) fn test_block_editor_hud_state(&self) -> Option<TestBlockEditorHudState<'_>> {
        let editor = self.screen.test_block()?;
        Some(TestBlockEditorHudState {
            message: &editor.message,
            mode: editor.mode,
            mouse_position: editor.mouse_position,
        })
    }

    pub(crate) fn open_test_block_editor(
        &mut self,
        world: &mut WorldStore,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        authoritative: TestBlockEditorState,
    ) {
        let editor = TestBlockEditorInputState {
            pos: authoritative.pos,
            message: TextField::with_value(
                authoritative.data.message.units(),
                MAX_TEST_BLOCK_MESSAGE_UTF16_UNITS,
            ),
            mode: authoritative.data.mode,
            mouse_position: None,
        };
        self.open_editor_screen(Screen::TestBlock(editor), world, counters, net_commands);
    }

    pub(crate) fn handle_test_block_editor_cursor_moved(
        &mut self,
        cursor_position: Option<PhysicalPosition<f64>>,
    ) -> bool {
        let Some(editor) = self.screen.test_block_mut() else {
            return false;
        };
        editor.mouse_position = cursor_position.and_then(physical_position_floor_i32);
        true
    }

    pub(crate) fn handle_test_block_editor_key(
        &mut self,
        code: KeyCode,
        pressed: bool,
        clipboard: &mut Option<&mut dyn DebugClipboard>,
    ) -> bool {
        if self.dialog_screen_is_open() || self.screen.test_block().is_none() {
            return false;
        }
        if !pressed {
            return true;
        }
        if code == KeyCode::Escape {
            self.screen.close();
            return true;
        }
        let modifiers = self.text_field_modifiers();
        let editor = self
            .screen
            .test_block_mut()
            .expect("checked test block editor");
        if editor.mode != TestBlockMode::Start {
            editor.message.key(code, modifiers, clipboard, None);
        }
        true
    }

    pub(crate) fn handle_test_block_editor_text(&mut self, text: &str) -> bool {
        if self.dialog_screen_is_open() {
            return false;
        }
        let Some(editor) = self.screen.test_block_mut() else {
            return false;
        };
        if editor.mode != TestBlockMode::Start {
            editor.message.type_text(text, None);
        }
        true
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn handle_test_block_editor_mouse_input(
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
        let Some(editor) = self.screen.test_block_mut() else {
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
            editor.message.click_bordered(
                layout.message,
                mouse_x,
                measurer.command_suggestion_glyphs(),
                shift_down,
            );
            return true;
        }
        if layout.mode.contains(mouse_x, mouse_y) {
            editor.mode = next_mode(editor.mode);
            return true;
        }
        if layout.cancel.contains(mouse_x, mouse_y) {
            self.screen.close();
            return true;
        }
        if layout.done.contains(mouse_x, mouse_y) {
            let packet = editor.packet();
            queue_set_test_block_command(counters, net_commands, packet);
            self.screen.close();
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
