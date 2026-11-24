use crate::control::NetCounters;
use bbb_client_net::NetCommand;
use bbb_client_render::hud_test_instance_block_editor_layout;
use bbb_client_world::{BlockPos, TestInstanceBlockEditorState, WorldStore};
use bbb_protocol::{
    normalize_resource_location,
    packets::{
        TestInstanceBlockAction, TestInstanceBlockActionKind, TestInstanceBlockRotation, Vec3i,
        MAX_TEST_INSTANCE_ID_UTF16_UNITS,
    },
    RichTextComponent,
};
use tokio::sync::mpsc;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton},
    keyboard::KeyCode,
};

use super::{
    physical_position_floor_i32, queue_test_instance_block_action_command, screen::Screen,
    text_field::TextField, ClientInputState, CommandSuggestionTextMeasurer, DebugClipboard,
};

const SIZE_FIELD_UTF16_UNITS: usize = 15;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TestInstanceBlockEditorFocus {
    Id,
    SizeX,
    SizeY,
    SizeZ,
}

impl TestInstanceBlockEditorFocus {
    pub(crate) fn index(self) -> usize {
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
pub(crate) struct TestInstanceBlockEditorInputState {
    pub(super) pos: BlockPos,
    pub(super) fields: [TextField; 4],
    pub(super) focus: TestInstanceBlockEditorFocus,
    pub(super) rotation: TestInstanceBlockRotation,
    pub(super) include_entities: bool,
    pub(super) error_message: Option<RichTextComponent>,
    pub(super) response_status: Option<RichTextComponent>,
    pub(super) local_invalid_id: bool,
    pub(super) mouse_position: Option<(i32, i32)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TestInstanceBlockEditorHudState<'a> {
    pub(crate) fields: &'a [TextField; 4],
    pub(crate) focus_index: usize,
    pub(crate) rotation: TestInstanceBlockRotation,
    pub(crate) include_entities: bool,
    pub(crate) error_message: Option<&'a RichTextComponent>,
    pub(crate) response_status: Option<&'a RichTextComponent>,
    pub(crate) local_invalid_id: bool,
    pub(crate) mouse_position: Option<(i32, i32)>,
    pub(crate) save_enabled: bool,
}

impl TestInstanceBlockEditorInputState {
    pub(crate) fn focus_index(&self) -> usize {
        self.focus.index()
    }

    fn field_mut(&mut self) -> &mut TextField {
        &mut self.fields[self.focus.index()]
    }

    fn normalized_test(&self) -> Option<String> {
        normalize_resource_location(self.fields[0].text()).ok()
    }

    fn size(&self) -> Vec3i {
        Vec3i {
            x: parse_size(&self.fields[1].text()),
            y: parse_size(&self.fields[2].text()),
            z: parse_size(&self.fields[3].text()),
        }
    }

    fn action(&self, action: TestInstanceBlockActionKind) -> TestInstanceBlockAction {
        TestInstanceBlockAction {
            pos: self.pos,
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
        self.screen.test_instance_block().is_some()
    }

    pub(crate) fn test_instance_block_editor_hud_state(
        &self,
    ) -> Option<TestInstanceBlockEditorHudState<'_>> {
        let editor = self.screen.test_instance_block()?;
        Some(TestInstanceBlockEditorHudState {
            fields: &editor.fields,
            focus_index: editor.focus.index(),
            rotation: editor.rotation,
            include_entities: editor.include_entities,
            error_message: editor.error_message.as_ref(),
            response_status: editor.response_status.as_ref(),
            local_invalid_id: editor.local_invalid_id,
            mouse_position: editor.mouse_position,
            save_enabled: editor.save_enabled(),
        })
    }

    pub(crate) fn open_test_instance_block_editor(
        &mut self,
        world: &mut WorldStore,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        authoritative: TestInstanceBlockEditorState,
    ) {
        let test = authoritative.data.test.unwrap_or_default();
        let mut editor = TestInstanceBlockEditorInputState {
            pos: authoritative.pos,
            fields: [
                TextField::with_text(&test, MAX_TEST_INSTANCE_ID_UTF16_UNITS),
                TextField::with_text(
                    &authoritative.data.size.x.to_string(),
                    SIZE_FIELD_UTF16_UNITS,
                ),
                TextField::with_text(
                    &authoritative.data.size.y.to_string(),
                    SIZE_FIELD_UTF16_UNITS,
                ),
                TextField::with_text(
                    &authoritative.data.size.z.to_string(),
                    SIZE_FIELD_UTF16_UNITS,
                ),
            ],
            focus: TestInstanceBlockEditorFocus::Id,
            rotation: authoritative.data.rotation,
            include_entities: !authoritative.data.ignore_entities,
            error_message: authoritative.data.error_message,
            response_status: None,
            local_invalid_id: false,
            mouse_position: None,
        };
        editor.local_invalid_id = editor.normalized_test().is_none();
        let init = editor.action(TestInstanceBlockActionKind::Init);
        self.open_editor_screen(
            Screen::TestInstanceBlock(editor),
            world,
            counters,
            net_commands,
        );
        queue_test_instance_block_action_command(counters, net_commands, init);
    }

    /// `TestInstanceBlockEditScreen.setStatus`.
    pub(crate) fn show_test_instance_block_status(
        &mut self,
        world: &WorldStore,
        status: bbb_client_world::TestInstanceBlockStatusState,
    ) {
        let Some(editor) = self.screen.test_instance_block_mut() else {
            return;
        };
        editor.error_message = world
            .test_instance_block_data_at(editor.pos)
            .and_then(|data| data.error_message.clone());
        editor.response_status = Some(status.status);
        editor.local_invalid_id = false;
        if let Some(size) = status.size {
            editor.fields[1].set_text(&size.x.to_string(), None);
            editor.fields[2].set_text(&size.y.to_string(), None);
            editor.fields[3].set_text(&size.z.to_string(), None);
        }
    }

    pub(crate) fn handle_test_instance_block_editor_cursor_moved(
        &mut self,
        cursor_position: Option<PhysicalPosition<f64>>,
    ) -> bool {
        let Some(editor) = self.screen.test_instance_block_mut() else {
            return false;
        };
        editor.mouse_position = cursor_position.and_then(physical_position_floor_i32);
        true
    }

    pub(crate) fn handle_test_instance_block_editor_key(
        &mut self,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        code: KeyCode,
        pressed: bool,
        clipboard: &mut Option<&mut dyn DebugClipboard>,
    ) -> bool {
        if self.dialog_screen_is_open() || self.screen.test_instance_block().is_none() {
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
            .test_instance_block_mut()
            .expect("checked test-instance editor");
        if code == KeyCode::Tab {
            let index = editor.focus.index();
            editor.focus = TestInstanceBlockEditorFocus::from_index(if modifiers.shift {
                index.wrapping_add(3)
            } else {
                index + 1
            });
            editor.field_mut().focus();
            return true;
        }
        let before = editor.fields[0].value().to_vec();
        editor.field_mut().key(code, modifiers, clipboard, None);
        if editor.fields[0].value() != before {
            self.queue_test_instance_block_query(counters, net_commands);
        }
        true
    }

    pub(crate) fn handle_test_instance_block_editor_text(
        &mut self,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        text: &str,
    ) -> bool {
        if self.dialog_screen_is_open() {
            return false;
        }
        let Some(editor) = self.screen.test_instance_block_mut() else {
            return false;
        };
        let before = editor.fields[0].value().to_vec();
        editor.field_mut().type_text(text, None);
        if editor.fields[0].value() != before {
            self.queue_test_instance_block_query(counters, net_commands);
        }
        true
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn handle_test_instance_block_editor_mouse_input(
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
        let Some(editor) = self.screen.test_instance_block_mut() else {
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
            self.screen.close();
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
            self.send_test_instance_block_action(counters, net_commands, action);
        }
        true
    }

    fn queue_test_instance_block_query(
        &mut self,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
    ) {
        let Some(editor) = self.screen.test_instance_block_mut() else {
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
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        action: TestInstanceBlockActionKind,
    ) {
        let Some(packet) = self
            .screen
            .test_instance_block()
            .map(|editor| editor.action(action))
        else {
            return;
        };
        queue_test_instance_block_action_command(counters, net_commands, packet);
        self.screen.close();
    }
}

fn parse_size(text: &str) -> i32 {
    text.parse::<i32>().unwrap_or(1).clamp(1, 48)
}
