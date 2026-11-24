use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommandConfirmationNarrationFrame {
    pub(crate) generation: u64,
    pub(crate) presentation: u64,
    pub(crate) retained: Vec<u64>,
    pub(crate) focused: Option<usize>,
    pub(crate) hovered: Option<usize>,
    pub(crate) unfocused_frame: ScreenNarrationFrame,
    pub(crate) focused_frames: [ScreenNarrationFrame; 2],
    pub(crate) hovered_frames: [ScreenNarrationFrame; 2],
}

#[derive(Debug, Default)]
pub(crate) struct CommandConfirmationNarrationRuntime {
    pub(crate) timing: ScreenNarrationRuntime,
    presentation: Option<u64>,
    last_focused: BTreeMap<u64, usize>,
    last_drawn_hover: BTreeMap<u64, Option<usize>>,
}

impl CommandConfirmationNarrationRuntime {
    pub(crate) fn tick(
        &mut self,
        frame: Option<&CommandConfirmationNarrationFrame>,
        now: Instant,
        last_input_type: LastInputType,
        narrator: Option<&mut dyn SystemNarrator>,
    ) {
        if frame.is_some_and(|frame| Some(frame.presentation) == self.presentation) {
            self.tick_inner(frame, now, last_input_type, narrator, true);
        }
    }

    pub(crate) fn initialize(
        &mut self,
        frame: &CommandConfirmationNarrationFrame,
        now: Instant,
        last_input_type: LastInputType,
        narrator: Option<&mut dyn SystemNarrator>,
    ) {
        self.tick_inner(Some(frame), now, last_input_type, narrator, false);
    }

    fn tick_inner(
        &mut self,
        frame: Option<&CommandConfirmationNarrationFrame>,
        now: Instant,
        last_input_type: LastInputType,
        narrator: Option<&mut dyn SystemNarrator>,
        drawn: bool,
    ) {
        let presentation = frame.map(|frame| frame.presentation);
        let initializing = self.presentation != presentation;
        if initializing {
            self.timing.reset();
            self.presentation = presentation;
        }
        if let Some(frame) = frame {
            self.last_focused
                .retain(|generation, _| frame.retained.contains(generation));
            self.last_drawn_hover
                .retain(|generation, _| frame.retained.contains(generation));
        }
        self.timing.tick_lazy(
            frame.map(|frame| {
                || {
                    let last = self.last_focused.get(&frame.generation).copied();
                    // Screen.init narrates before the next AbstractWidget extraction.
                    let hovered = if initializing {
                        self.last_drawn_hover
                            .get(&frame.generation)
                            .copied()
                            .flatten()
                    } else {
                        frame.hovered
                    };
                    // Screen.findNarratableWidget chooses a new focus before a hovered widget.
                    if let Some(focused) = frame.focused.filter(|focused| Some(*focused) != last) {
                        self.last_focused.insert(frame.generation, focused);
                        return frame.focused_frames[focused].clone();
                    }
                    if let Some(hovered) = hovered.filter(|hovered| Some(*hovered) != frame.focused)
                    {
                        return frame.hovered_frames[hovered].clone();
                    }
                    if let Some(focused) = frame.focused {
                        self.last_focused.insert(frame.generation, focused);
                        frame.focused_frames[focused].clone()
                    } else {
                        frame.unfocused_frame.clone()
                    }
                }
            }),
            now,
            last_input_type,
            narrator,
        );
        if drawn {
            if let Some(frame) = frame {
                self.last_drawn_hover
                    .insert(frame.generation, frame.hovered);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Speech(Vec<String>);
    impl SystemNarrator for Speech {
        fn say_system_now(&mut self, text: &str) {
            self.0.push(text.into());
        }
        fn say_chat_queued(&mut self, _: &str) {}
        fn say_system_chat_queued(&mut self, _: &str) {}
        fn say_system_queued(&mut self, _: &str) {}
        fn update_status(&mut self, _: NarratorStatus) {}
    }

    fn frame(generation: u64, presentation: u64) -> CommandConfirmationNarrationFrame {
        let button = |label, usage| {
            let mut frame = ScreenNarrationFrame::default();
            frame.add(NarratedElementKind::Title, 0, "Run command?. Full message");
            frame.add(NarratedElementKind::Title, 1, label);
            frame.add(NarratedElementKind::Usage, 1, usage);
            frame
        };
        CommandConfirmationNarrationFrame {
            generation,
            presentation,
            retained: vec![generation],
            focused: Some(0),
            unfocused_frame: button("", "Tab to navigate"),
            hovered: Some(1),
            focused_frames: [button("Run", "Enter"), button("Back", "Enter")],
            hovered_frames: [button("Run", "Click"), button("Back", "Click")],
        }
    }

    #[test]
    fn command_confirmation_narration_does_not_infer_initialization_from_publication() {
        let start = Instant::now();
        let mut runtime = CommandConfirmationNarrationRuntime::default();
        let mut speech = Speech::default();
        let first = frame(1, 1);
        let second = frame(2, 2);
        runtime.tick(Some(&first), start, LastInputType::Mouse, Some(&mut speech));
        assert!(speech.0.is_empty());
        runtime.initialize(&first, start, LastInputType::Mouse, Some(&mut speech));
        runtime.initialize(&second, start, LastInputType::Mouse, Some(&mut speech));
        runtime.tick(Some(&first), start, LastInputType::Mouse, Some(&mut speech));
        runtime.tick(None, start, LastInputType::Mouse, Some(&mut speech));
        runtime.tick(
            Some(&second),
            start,
            LastInputType::Mouse,
            Some(&mut speech),
        );
        assert_eq!(speech.0.len(), 2);
        assert_eq!(runtime.presentation, Some(2));
    }

    #[test]
    fn command_confirmation_initial_narration_precedes_drawing_and_return_preserves_hover() {
        let start = Instant::now();
        let mut runtime = CommandConfirmationNarrationRuntime::default();
        let mut speech = Speech::default();
        let mut screen = frame(1, 1);
        screen.focused = None;
        runtime.initialize(&screen, start, LastInputType::Mouse, Some(&mut speech));
        assert_eq!(speech.0, ["Run command?. Full message. Tab to navigate"]);
        runtime
            .timing
            .after_input(ScreenNarrationInput::MouseAction, start);
        runtime.tick(
            Some(&screen),
            start + Duration::from_millis(201),
            LastInputType::Mouse,
            Some(&mut speech),
        );
        assert_eq!(speech.0[1], "Back. Click");
        runtime.tick(
            None,
            start + Duration::from_secs(1),
            LastInputType::Mouse,
            Some(&mut speech),
        );
        screen.presentation = 2;
        screen.hovered = Some(0);
        runtime.initialize(
            &screen,
            start + Duration::from_secs(2),
            LastInputType::Mouse,
            Some(&mut speech),
        );
        assert_eq!(speech.0[2], "Run command?. Full message. Back. Click");
        runtime.timing.after_input(
            ScreenNarrationInput::MouseAction,
            start + Duration::from_secs(2),
        );
        runtime.tick(
            Some(&screen),
            start + Duration::from_millis(2201),
            LastInputType::Mouse,
            Some(&mut speech),
        );
        assert_eq!(speech.0[3], "Run");
        screen.hovered = None;
        runtime.tick(
            Some(&screen),
            start + Duration::from_secs(3),
            LastInputType::Mouse,
            Some(&mut speech),
        );
        assert_eq!(speech.0.len(), 4);
        assert_eq!(runtime.last_drawn_hover.get(&1), Some(&None));
    }

    #[test]
    fn command_confirmation_narration_without_focus_uses_hover_or_screen_usage() {
        let start = Instant::now();
        let mut runtime = CommandConfirmationNarrationRuntime::default();
        let mut speech = Speech::default();
        let mut screen = frame(1, 1);
        screen.focused = None;
        screen.hovered = None;
        runtime.initialize(&screen, start, LastInputType::Mouse, Some(&mut speech));
        assert_eq!(speech.0, ["Run command?. Full message. Tab to navigate"]);
        assert!(runtime.last_focused.is_empty());
        screen.hovered = Some(1);
        runtime
            .timing
            .after_input(ScreenNarrationInput::MouseAction, start);
        runtime.tick(
            Some(&screen),
            start + Duration::from_millis(201),
            LastInputType::Mouse,
            Some(&mut speech),
        );
        assert_eq!(speech.0[1], "Back. Click");
        assert!(runtime.last_focused.is_empty());
        screen.focused = Some(0);
        runtime.timing.after_input(
            ScreenNarrationInput::KeyboardAction,
            start + Duration::from_secs(1),
        );
        runtime.tick(
            Some(&screen),
            start + Duration::from_millis(1201),
            LastInputType::KeyboardTab,
            Some(&mut speech),
        );
        assert_eq!(speech.0[2], "Run. Enter");
    }

    #[test]
    fn command_confirmation_narration_selects_widgets_only_when_due_and_restores_parent() {
        let start = Instant::now();
        let mut runtime = CommandConfirmationNarrationRuntime::default();
        let mut speech = Speech::default();
        let mut parent = frame(1, 1);
        runtime.initialize(&parent, start, LastInputType::Mouse, Some(&mut speech));
        assert!(speech.0[0].contains("Run. Enter"));
        runtime
            .timing
            .after_input(ScreenNarrationInput::MouseAction, start);
        runtime.tick(
            Some(&parent),
            start + Duration::from_millis(200),
            LastInputType::Mouse,
            Some(&mut speech),
        );
        assert_eq!(speech.0.len(), 1);
        runtime.tick(
            Some(&parent),
            start + Duration::from_millis(201),
            LastInputType::Mouse,
            Some(&mut speech),
        );
        assert_eq!(speech.0[1], "Back. Click");
        parent.focused = Some(1);
        parent.hovered = Some(0);
        runtime.timing.after_input(
            ScreenNarrationInput::KeyboardAction,
            start + Duration::from_secs(1),
        );
        runtime.tick(
            Some(&parent),
            start + Duration::from_millis(1201),
            LastInputType::Mouse,
            Some(&mut speech),
        );
        assert_eq!(speech.0[2], "Enter");
        let mut child = frame(2, 2);
        child.retained = vec![1, 2];
        runtime.initialize(
            &child,
            start + Duration::from_secs(2),
            LastInputType::Mouse,
            Some(&mut speech),
        );
        parent.presentation = 3;
        runtime.initialize(
            &parent,
            start + Duration::from_secs(3),
            LastInputType::Mouse,
            Some(&mut speech),
        );
        assert_eq!(
            speech.0.last().unwrap(),
            "Run command?. Full message. Run. Click"
        );
        assert_eq!(runtime.last_focused.len(), 1);
        runtime.tick(
            None,
            start + Duration::from_secs(4),
            LastInputType::Mouse,
            Some(&mut speech),
        );
        runtime.tick(
            Some(&parent),
            start + Duration::from_secs(5),
            LastInputType::Mouse,
            Some(&mut speech),
        );
        assert_eq!(speech.0.len(), 5);
    }
}
