use super::*;

impl DialogState {
    pub fn is_resource_pack_prompt(&self) -> bool {
        self.holder_kind == "resource_pack_consent"
    }

    pub(crate) fn resource_pack_prompt(
        latest_sequence: u64,
        required: bool,
        prompt: Option<&bbb_protocol::RichTextComponent>,
        resolve_text: impl Fn(&NbtSummaryValue) -> Vec<StyledTextRun>,
    ) -> Self {
        let (title, body, yes, no) = if required {
            (
                "multiplayer.requiredTexturePrompt.line1",
                "multiplayer.requiredTexturePrompt.line2",
                "gui.proceed",
                "menu.disconnect",
            )
        } else {
            (
                "multiplayer.texturePrompt.line1",
                "multiplayer.texturePrompt.line2",
                "gui.yes",
                "gui.no",
            )
        };
        let mut header = translated_component(body);
        if required {
            let NbtSummaryValue::Compound(fields) = &mut header else {
                unreachable!()
            };
            fields.extend([
                NbtSummaryEntry {
                    name: "color".into(),
                    value: NbtSummaryValue::string("yellow"),
                },
                NbtSummaryEntry {
                    name: "bold".into(),
                    value: NbtSummaryValue::Byte(1),
                },
            ]);
        }
        let message = match prompt {
            Some(prompt) => NbtSummaryValue::Compound(vec![
                NbtSummaryEntry {
                    name: "translate".into(),
                    value: NbtSummaryValue::string("multiplayer.texturePrompt.serverPrompt"),
                },
                NbtSummaryEntry {
                    name: "with".into(),
                    value: NbtSummaryValue::List(vec![header, prompt.component().clone()]),
                },
            ]),
            None => header,
        };
        let text = |component| {
            let runs = resolve_text(&component);
            DialogTextState {
                text: styled_runs_summary_text(&runs),
                runs,
                component,
            }
        };
        let button = |label: &str, accepted| DialogButtonState {
            label: text(translated_component(label)),
            tooltip: None,
            width: 150,
            action: None,
            action_data: Some(DialogActionState::ResourcePackConsent {
                accepted,
                latest_sequence,
            }),
        };
        Self {
            holder_kind: "resource_pack_consent".to_owned(),
            registry_id: None,
            resource_id: None,
            dialog: None,
            resolution_error: None,
            semantic_error: None,
            data: Some(DialogDataState::Confirmation {
                common: CommonDialogDataState {
                    title: text(translated_component(title)),
                    external_title: None,
                    can_close_with_escape: true,
                    pause: false,
                    after_action: "close".into(),
                    body: Vec::new(),
                    body_data: vec![DialogBodyState::PlainMessage {
                        message: DialogPlainMessageState {
                            contents: text(message),
                            width: 320,
                        },
                    }],
                    inputs: Vec::new(),
                    input_data: Vec::new(),
                },
                yes: button(yes, true),
                no: button(no, false),
            }),
        }
    }
}
