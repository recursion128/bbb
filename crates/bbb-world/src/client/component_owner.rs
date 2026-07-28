use bbb_protocol::{
    packets::{
        BossEvent, BossEventOperation, ChatTypeBound, ChatTypeHolder, CommandSuggestions,
        DisguisedChat, MapItemData, NumberFormatSummary, OpenScreen, PlayerChat, PlayerCombatKill,
        PlayerInfoUpdate, SetActionBarText, SetObjective, SetPlayerTeam, SetScore, SetSubtitleText,
        SetTitleText, SystemChat, TabList, TestInstanceBlockStatus, UpdateAdvancements,
    },
    NbtSummaryValue, RichTextComponent, RichTextStyle,
};

use crate::{FrozenRegistryAccess, WorldStore};

use super::{validate_command_style, validate_rich_component_with_registry};

pub(crate) trait PacketComponentOwner {
    fn validate_owned_components(
        &self,
        registry_access: FrozenRegistryAccess<'_>,
    ) -> Result<(), String>;
}

impl WorldStore {
    pub(crate) fn preflight_packet_components<T: PacketComponentOwner>(
        &mut self,
        packet: &T,
        family: &'static str,
    ) -> bool {
        let result = packet.validate_owned_components(self.frozen_registry_access());
        match result {
            Ok(()) => true,
            Err(error) => {
                self.record_apply_error(
                    "packet_component",
                    format!("invalid {family} packet component: {error}"),
                );
                false
            }
        }
    }
}

fn validate_component(
    component: &RichTextComponent,
    field: &str,
    registry_access: FrozenRegistryAccess<'_>,
) -> Result<(), String> {
    validate_component_nbt(component.component(), field, registry_access)
}

fn validate_component_nbt(
    component: &NbtSummaryValue,
    field: &str,
    registry_access: FrozenRegistryAccess<'_>,
) -> Result<(), String> {
    validate_rich_component_with_registry(component, registry_access)
        .map_err(|error| format!("{field}: {error}"))
}

fn validate_optional_component_nbt(
    component: Option<&NbtSummaryValue>,
    field: &str,
    registry_access: FrozenRegistryAccess<'_>,
) -> Result<(), String> {
    component
        .map(|component| validate_component_nbt(component, field, registry_access))
        .transpose()
        .map(|_| ())
}

fn validate_style(
    style: &RichTextStyle,
    field: &str,
    registry_access: FrozenRegistryAccess<'_>,
) -> Result<(), String> {
    validate_command_style(style.component(), registry_access)
        .map_err(|error| format!("{field}: {error}"))
}

fn validate_number_format(
    number_format: Option<&NumberFormatSummary>,
    field: &str,
    registry_access: FrozenRegistryAccess<'_>,
) -> Result<(), String> {
    match number_format {
        None | Some(NumberFormatSummary::Blank) => Ok(()),
        Some(NumberFormatSummary::Styled(style)) => validate_style(style, field, registry_access),
        Some(NumberFormatSummary::Fixed(component)) => {
            validate_component(component, field, registry_access)
        }
    }
}

fn validate_chat_type_bound(
    chat_type: &ChatTypeBound,
    field: &str,
    registry_access: FrozenRegistryAccess<'_>,
) -> Result<(), String> {
    validate_optional_component_nbt(
        chat_type.name_component.as_ref(),
        &format!("{field}.name"),
        registry_access,
    )?;
    validate_optional_component_nbt(
        chat_type.target_name_component.as_ref(),
        &format!("{field}.target_name"),
        registry_access,
    )?;
    if let ChatTypeHolder::Direct { chat, narration } = &chat_type.chat_type {
        validate_style(&chat.style, &format!("{field}.chat.style"), registry_access)?;
        validate_style(
            &narration.style,
            &format!("{field}.narration.style"),
            registry_access,
        )?;
    }
    Ok(())
}

impl PacketComponentOwner for BossEvent {
    fn validate_owned_components(
        &self,
        registry_access: FrozenRegistryAccess<'_>,
    ) -> Result<(), String> {
        match &self.operation {
            BossEventOperation::Add { name, .. } | BossEventOperation::UpdateName { name } => {
                validate_component(name, "name", registry_access)
            }
            BossEventOperation::Remove
            | BossEventOperation::UpdateProgress { .. }
            | BossEventOperation::UpdateStyle { .. }
            | BossEventOperation::UpdateProperties { .. } => Ok(()),
        }
    }
}

impl PacketComponentOwner for TabList {
    fn validate_owned_components(
        &self,
        registry_access: FrozenRegistryAccess<'_>,
    ) -> Result<(), String> {
        validate_component(&self.header, "header", registry_access)?;
        validate_component(&self.footer, "footer", registry_access)
    }
}

impl PacketComponentOwner for SystemChat {
    fn validate_owned_components(
        &self,
        registry_access: FrozenRegistryAccess<'_>,
    ) -> Result<(), String> {
        validate_component(&self.content, "content", registry_access)
    }
}

macro_rules! impl_content_component_owner {
    ($($packet:ty),+ $(,)?) => {
        $(
            impl PacketComponentOwner for $packet {
                fn validate_owned_components(
                    &self,
                    registry_access: FrozenRegistryAccess<'_>,
                ) -> Result<(), String> {
                    validate_component(&self.content, "content", registry_access)
                }
            }
        )+
    };
}

impl_content_component_owner!(SetActionBarText, SetTitleText, SetSubtitleText);

impl PacketComponentOwner for PlayerCombatKill {
    fn validate_owned_components(
        &self,
        registry_access: FrozenRegistryAccess<'_>,
    ) -> Result<(), String> {
        validate_component(&self.message, "message", registry_access)
    }
}

impl PacketComponentOwner for OpenScreen {
    fn validate_owned_components(
        &self,
        registry_access: FrozenRegistryAccess<'_>,
    ) -> Result<(), String> {
        validate_component(&self.title, "title", registry_access)
    }
}

impl PacketComponentOwner for SetObjective {
    fn validate_owned_components(
        &self,
        registry_access: FrozenRegistryAccess<'_>,
    ) -> Result<(), String> {
        let Some(parameters) = &self.parameters else {
            return Ok(());
        };
        validate_component(&parameters.display_name, "display_name", registry_access)?;
        validate_number_format(
            parameters.number_format.as_ref(),
            "number_format",
            registry_access,
        )
    }
}

impl PacketComponentOwner for SetScore {
    fn validate_owned_components(
        &self,
        registry_access: FrozenRegistryAccess<'_>,
    ) -> Result<(), String> {
        if let Some(display) = &self.display {
            validate_component(display, "display", registry_access)?;
        }
        validate_number_format(
            self.number_format.as_ref(),
            "number_format",
            registry_access,
        )
    }
}

impl PacketComponentOwner for SetPlayerTeam {
    fn validate_owned_components(
        &self,
        registry_access: FrozenRegistryAccess<'_>,
    ) -> Result<(), String> {
        let Some(parameters) = &self.parameters else {
            return Ok(());
        };
        validate_component(&parameters.display_name, "display_name", registry_access)?;
        validate_component(&parameters.player_prefix, "player_prefix", registry_access)?;
        validate_component(&parameters.player_suffix, "player_suffix", registry_access)
    }
}

impl PacketComponentOwner for DisguisedChat {
    fn validate_owned_components(
        &self,
        registry_access: FrozenRegistryAccess<'_>,
    ) -> Result<(), String> {
        validate_optional_component_nbt(
            self.message_component.as_ref(),
            "message",
            registry_access,
        )?;
        validate_chat_type_bound(&self.chat_type, "chat_type", registry_access)
    }
}

impl PacketComponentOwner for PlayerChat {
    fn validate_owned_components(
        &self,
        registry_access: FrozenRegistryAccess<'_>,
    ) -> Result<(), String> {
        validate_optional_component_nbt(
            self.unsigned_content_component.as_ref(),
            "unsigned_content",
            registry_access,
        )?;
        validate_chat_type_bound(&self.chat_type, "chat_type", registry_access)
    }
}

impl PacketComponentOwner for CommandSuggestions {
    fn validate_owned_components(
        &self,
        registry_access: FrozenRegistryAccess<'_>,
    ) -> Result<(), String> {
        for (index, suggestion) in self.suggestions.iter().enumerate() {
            validate_optional_component_nbt(
                suggestion.tooltip_component.as_ref(),
                &format!("suggestions[{index}].tooltip"),
                registry_access,
            )?;
        }
        Ok(())
    }
}

impl PacketComponentOwner for UpdateAdvancements {
    fn validate_owned_components(
        &self,
        registry_access: FrozenRegistryAccess<'_>,
    ) -> Result<(), String> {
        for (index, advancement) in self.added.iter().enumerate() {
            let Some(display) = &advancement.display else {
                continue;
            };
            validate_component(
                &display.title,
                &format!("added[{index}].display.title"),
                registry_access,
            )?;
            validate_component(
                &display.description,
                &format!("added[{index}].display.description"),
                registry_access,
            )?;
        }
        Ok(())
    }
}

impl PacketComponentOwner for PlayerInfoUpdate {
    fn validate_owned_components(
        &self,
        registry_access: FrozenRegistryAccess<'_>,
    ) -> Result<(), String> {
        for (index, entry) in self.entries.iter().enumerate() {
            if let Some(display_name) = &entry.display_name {
                validate_component(
                    display_name,
                    &format!("entries[{index}].display_name"),
                    registry_access,
                )?;
            }
        }
        Ok(())
    }
}

impl PacketComponentOwner for MapItemData {
    fn validate_owned_components(
        &self,
        registry_access: FrozenRegistryAccess<'_>,
    ) -> Result<(), String> {
        for (index, decoration) in self.decorations.iter().flatten().enumerate() {
            if let Some(name) = &decoration.name {
                validate_component(name, &format!("decorations[{index}].name"), registry_access)?;
            }
        }
        Ok(())
    }
}

impl PacketComponentOwner for TestInstanceBlockStatus {
    fn validate_owned_components(
        &self,
        registry_access: FrozenRegistryAccess<'_>,
    ) -> Result<(), String> {
        validate_component(&self.status, "status", registry_access)
    }
}

#[cfg(test)]
mod tests {
    use bbb_protocol::menu_types::VANILLA_MENU_TYPE_GENERIC_9X3_ID;
    use bbb_protocol::packets::{
        AdvancementDisplaySummary, AdvancementFrameType, AdvancementIconSummary,
        AdvancementSummary, BossBarColor, BossBarOverlay, BossEventFlags, ChatFormatting,
        ChatTypeDecorationSummary, CommandSuggestion, DataComponentPatchSummary, FilterMask,
        FilterMaskKind, GameType, MapColorPatch, MapDecoration, ObjectiveRenderType,
        PlayerInfoAction, PlayerInfoEntry, PlayerTeamMethod, PlayerTeamParameters,
        SetObjectiveMethod, SetObjectiveParameters, SignedMessageBody, TeamCollisionRule,
        TeamVisibility,
    };
    use bbb_protocol::{NbtSummaryEntry, StyledTextRun};
    use uuid::Uuid;

    use crate::RegistryPacketEntry;

    use super::super::dialog::DIALOG_REGISTRY;
    use super::*;

    #[test]
    fn owner_inventory_rejects_every_direct_component_carrier() {
        let store = WorldStore::new();
        let registry_access = store.frozen_registry_access();
        let invalid = invalid_component();

        assert_invalid(
            &BossEvent {
                id: Uuid::from_u128(1),
                operation: BossEventOperation::Add {
                    name: Box::new(invalid.clone()),
                    progress: 1.0,
                    color: BossBarColor::White,
                    overlay: BossBarOverlay::Progress,
                    flags: BossEventFlags::default(),
                },
            },
            registry_access,
            "name",
        );
        assert_invalid(
            &TabList {
                header: invalid.clone(),
                footer: RichTextComponent::plain("footer"),
            },
            registry_access,
            "header",
        );
        assert_invalid(
            &SystemChat {
                content: Box::new(invalid.clone()),
                overlay: false,
            },
            registry_access,
            "content",
        );
        assert_invalid(
            &SetActionBarText {
                content: invalid.clone(),
            },
            registry_access,
            "content",
        );
        assert_invalid(
            &SetTitleText {
                content: invalid.clone(),
            },
            registry_access,
            "content",
        );
        assert_invalid(
            &SetSubtitleText {
                content: invalid.clone(),
            },
            registry_access,
            "content",
        );
        assert_invalid(
            &PlayerCombatKill {
                player_id: 7,
                message: invalid.clone(),
            },
            registry_access,
            "message",
        );
        assert_invalid(
            &OpenScreen {
                container_id: 2,
                menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X3_ID,
                title: invalid.clone(),
            },
            registry_access,
            "title",
        );
        assert_invalid(
            &SetObjective {
                objective_name: "points".to_string(),
                method: SetObjectiveMethod::Add.into(),
                parameters: Some(SetObjectiveParameters {
                    display_name: invalid.clone(),
                    render_type: ObjectiveRenderType::Integer,
                    number_format: None,
                }),
            },
            registry_access,
            "display_name",
        );
        assert_invalid(
            &SetScore {
                owner: "Alice".to_string(),
                objective_name: "points".to_string(),
                score: 1,
                display: None,
                number_format: Some(NumberFormatSummary::Styled(invalid_style())),
            },
            registry_access,
            "number_format",
        );
        assert_invalid(
            &SetPlayerTeam {
                name: "red".to_string(),
                method: PlayerTeamMethod::Add.into(),
                parameters: Some(team_parameters(invalid.clone())),
                players: vec!["Alice".to_string()],
            },
            registry_access,
            "player_suffix",
        );
        assert_invalid(
            &DisguisedChat {
                message: "hello".to_string(),
                message_styled: Vec::new(),
                message_component: Some(invalid.component().clone()),
                chat_type: plain_chat_type(),
            },
            registry_access,
            "message",
        );
        assert_invalid(
            &player_chat(Some(invalid.component().clone())),
            registry_access,
            "unsigned_content",
        );
        assert_invalid(
            &DisguisedChat {
                message: "hello".to_string(),
                message_styled: Vec::new(),
                message_component: None,
                chat_type: direct_chat_type(invalid_style()),
            },
            registry_access,
            "chat_type.chat.style",
        );
        assert_invalid(
            &CommandSuggestions {
                id: 0,
                start: 0,
                length: 0,
                suggestions: vec![command_suggestion(
                    "open",
                    Some(invalid.component().clone()),
                )],
            },
            registry_access,
            "suggestions[0].tooltip",
        );
        assert_invalid(
            &UpdateAdvancements {
                reset: false,
                added: vec![advancement("root", invalid.clone())],
                removed: Vec::new(),
                progress: Vec::new(),
                show_advancements: false,
            },
            registry_access,
            "display.title",
        );
        assert_invalid(
            &PlayerInfoUpdate {
                actions: vec![PlayerInfoAction::UpdateDisplayName],
                entries: vec![player_info_entry(invalid.clone())],
            },
            registry_access,
            "display_name",
        );
        assert_invalid(
            &MapItemData {
                map_id: 1,
                scale: 0,
                locked: false,
                decorations: Some(vec![map_decoration(Some(invalid.clone()))]),
                color_patch: None,
            },
            registry_access,
            "decorations[0].name",
        );
        assert_invalid(
            &TestInstanceBlockStatus {
                status: invalid,
                size: None,
            },
            registry_access,
            "status",
        );
    }

    #[test]
    fn session_registry_and_direct_dialog_holders_validate_before_hud_mutation() {
        let mut store = WorldStore::new();
        store.record_registry_entries(
            DIALOG_REGISTRY,
            0,
            vec![
                RegistryPacketEntry::with_known_pack_data_for_test(
                    "example:first",
                    br#"{"type":"minecraft:notice","title":"First"}"#.to_vec(),
                ),
                RegistryPacketEntry::with_known_pack_data_for_test(
                    "example:target",
                    br#"{"type":"minecraft:notice","title":"Target"}"#.to_vec(),
                ),
            ],
        );
        assert_eq!(
            store.registry_resource_id_by_holder_id(DIALOG_REGISTRY, 1),
            Some("example:target")
        );

        let referenced = dialog_component(NbtSummaryValue::String("example:target".to_string()));
        store.apply_title_text(SetTitleText {
            content: referenced.clone(),
        });
        assert_eq!(store.client_hud().title.title.as_ref(), Some(&referenced));
        let retained = store.publication_snapshot();

        store.apply_title_text(SetTitleText {
            content: invalid_component(),
        });
        assert_eq!(store.client_hud().title.title.as_ref(), Some(&referenced));
        assert_eq!(
            retained.client_hud().title.title.as_ref(),
            Some(&referenced)
        );
        assert_packet_component_diagnostic(&store, "example:missing");

        let direct = dialog_component(NbtSummaryValue::Compound(vec![
            entry(
                "type",
                NbtSummaryValue::String("minecraft:notice".to_string()),
            ),
            entry("title", NbtSummaryValue::String("Inline".to_string())),
        ]));
        store.apply_title_text(SetTitleText {
            content: direct.clone(),
        });
        assert_eq!(store.client_hud().title.title.as_ref(), Some(&direct));
        assert_eq!(
            retained.client_hud().title.title.as_ref(),
            Some(&referenced)
        );
    }

    #[test]
    fn scoreboard_multi_component_update_is_atomic_and_recovers() {
        let mut store = WorldStore::new();
        assert!(store.apply_set_player_team(SetPlayerTeam {
            name: "red".to_string(),
            method: PlayerTeamMethod::Add.into(),
            parameters: Some(team_parameters(RichTextComponent::plain("before"))),
            players: vec!["Alice".to_string()],
        }));
        let expected = store.scoreboard().teams.clone();

        assert!(!store.apply_set_player_team(SetPlayerTeam {
            name: "red".to_string(),
            method: PlayerTeamMethod::Change.into(),
            parameters: Some(team_parameters(invalid_component())),
            players: Vec::new(),
        }));
        assert_eq!(store.scoreboard().teams, expected);
        assert_packet_component_diagnostic(&store, "player_suffix");

        assert!(store.apply_set_player_team(SetPlayerTeam {
            name: "red".to_string(),
            method: PlayerTeamMethod::Change.into(),
            parameters: Some(team_parameters(RichTextComponent::plain("after"))),
            players: Vec::new(),
        }));
        assert_eq!(
            store
                .scoreboard()
                .teams
                .get("red")
                .and_then(|team| team.parameters.as_ref())
                .map(|parameters| parameters.player_suffix.as_str()),
            Some("after")
        );
    }

    #[test]
    fn chat_and_command_rejection_preserves_sequence_and_pending_request() {
        let mut store = WorldStore::new();
        assert_eq!(
            store.apply_player_chat(player_chat(Some(invalid_component().component().clone()))),
            None
        );
        assert_eq!(store.client_chat().expected_player_chat_global_index, 0);
        assert!(store.client_chat().messages.is_empty());

        assert_eq!(
            store.apply_player_chat(player_chat(Some(NbtSummaryValue::String(
                "safe".to_string()
            )))),
            None
        );
        assert_eq!(store.client_chat().expected_player_chat_global_index, 1);
        assert_eq!(store.client_chat().messages.len(), 1);
        assert_eq!(store.counters().player_chat_index_mismatches, 0);

        let request = store.next_command_suggestion_request("/say");
        let invalid = CommandSuggestions {
            id: request.id,
            start: 0,
            length: 4,
            suggestions: vec![
                command_suggestion("safe", Some(NbtSummaryValue::String("safe".to_string()))),
                command_suggestion("invalid", Some(invalid_component().component().clone())),
            ],
        };
        assert_eq!(store.apply_command_suggestions(invalid), None);
        assert_eq!(
            store.command_suggestions().pending_request_id,
            Some(request.id)
        );
        assert!(store.command_suggestions().by_id.is_empty());

        assert!(store
            .apply_command_suggestions(CommandSuggestions {
                id: request.id,
                start: 0,
                length: 4,
                suggestions: vec![command_suggestion(
                    "recovered",
                    Some(NbtSummaryValue::String("recovered".to_string())),
                )],
            })
            .is_some());
        assert_eq!(store.command_suggestions().pending_request_id, None);
        assert_eq!(
            store
                .last_command_suggestions()
                .and_then(|result| result.suggestions.first())
                .map(|suggestion| suggestion.text.as_str()),
            Some("recovered")
        );
    }

    #[test]
    fn advancement_and_map_batches_reject_atomically_and_keep_old_publication() {
        let mut store = WorldStore::new();
        store.apply_update_advancements(UpdateAdvancements {
            reset: false,
            added: vec![advancement(
                "old",
                RichTextComponent::plain("Old advancement"),
            )],
            removed: Vec::new(),
            progress: Vec::new(),
            show_advancements: false,
        });
        assert!(store.advancements().advancements.contains_key("old"));

        store.apply_update_advancements(UpdateAdvancements {
            reset: true,
            added: vec![advancement("invalid", invalid_component())],
            removed: vec!["old".to_string()],
            progress: Vec::new(),
            show_advancements: false,
        });
        assert!(store.advancements().advancements.contains_key("old"));
        assert!(!store.advancements().advancements.contains_key("invalid"));

        assert!(store.apply_map_item_data(MapItemData {
            map_id: 9,
            scale: 0,
            locked: false,
            decorations: Some(vec![map_decoration(Some(RichTextComponent::plain("Old")))]),
            color_patch: Some(MapColorPatch {
                start_x: 0,
                start_y: 0,
                width: 1,
                height: 1,
                colors: vec![3],
            }),
        }));
        let expected = store.map_item(9).unwrap().clone();
        let retained = store.publication_snapshot();

        assert!(!store.apply_map_item_data(MapItemData {
            map_id: 9,
            scale: 4,
            locked: true,
            decorations: Some(vec![
                map_decoration(Some(RichTextComponent::plain("New"))),
                map_decoration(Some(invalid_component())),
            ]),
            color_patch: Some(MapColorPatch {
                start_x: 0,
                start_y: 0,
                width: 1,
                height: 1,
                colors: vec![8],
            }),
        }));
        assert_eq!(store.map_item(9), Some(&expected));
        assert_eq!(retained.map_item(9), Some(&expected));

        assert!(store.apply_map_item_data(MapItemData {
            map_id: 9,
            scale: 4,
            locked: true,
            decorations: Some(vec![map_decoration(Some(RichTextComponent::plain("New")))]),
            color_patch: Some(MapColorPatch {
                start_x: 0,
                start_y: 0,
                width: 1,
                height: 1,
                colors: vec![8],
            }),
        }));
        assert_ne!(store.map_item(9), Some(&expected));
        assert_eq!(retained.map_item(9), Some(&expected));
    }

    fn assert_invalid<T: PacketComponentOwner>(
        packet: &T,
        registry_access: FrozenRegistryAccess<'_>,
        expected_field: &str,
    ) {
        let error = packet
            .validate_owned_components(registry_access)
            .expect_err("owner-sensitive component must reject a missing dialog");
        assert!(
            error.contains(expected_field),
            "expected {expected_field:?} in {error:?}"
        );
        assert!(error.contains("example:missing"), "{error}");
    }

    fn invalid_component() -> RichTextComponent {
        dialog_component(NbtSummaryValue::String("example:missing".to_string()))
    }

    fn dialog_component(dialog: NbtSummaryValue) -> RichTextComponent {
        RichTextComponent::try_from_component_nbt(NbtSummaryValue::Compound(vec![
            entry("text", NbtSummaryValue::String("Open".to_string())),
            entry(
                "click_event",
                NbtSummaryValue::Compound(vec![
                    entry("action", NbtSummaryValue::String("show_dialog".to_string())),
                    entry("dialog", dialog),
                ]),
            ),
        ]))
        .unwrap()
    }

    fn invalid_style() -> RichTextStyle {
        RichTextStyle::try_from_style_nbt(NbtSummaryValue::Compound(vec![entry(
            "click_event",
            NbtSummaryValue::Compound(vec![
                entry("action", NbtSummaryValue::String("show_dialog".to_string())),
                entry(
                    "dialog",
                    NbtSummaryValue::String("example:missing".to_string()),
                ),
            ]),
        )]))
        .unwrap()
    }

    fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
        NbtSummaryEntry {
            name: name.to_string(),
            value,
        }
    }

    fn plain_chat_type() -> ChatTypeBound {
        ChatTypeBound {
            chat_type: ChatTypeHolder::Registry { id: 0 },
            name: "Alice".to_string(),
            name_styled: vec![StyledTextRun {
                text: "Alice".to_string(),
                style: Default::default(),
            }],
            name_component: None,
            target_name: None,
            target_name_styled: None,
            target_name_component: None,
        }
    }

    fn direct_chat_type(style: RichTextStyle) -> ChatTypeBound {
        ChatTypeBound {
            chat_type: ChatTypeHolder::Direct {
                chat: ChatTypeDecorationSummary {
                    translation_key: "chat.type.text".to_string(),
                    parameters: vec![0, 1],
                    style,
                },
                narration: ChatTypeDecorationSummary {
                    translation_key: "chat.type.text.narrate".to_string(),
                    parameters: vec![0, 1],
                    style: RichTextStyle::default(),
                },
            },
            ..plain_chat_type()
        }
    }

    fn player_chat(unsigned_content_component: Option<NbtSummaryValue>) -> PlayerChat {
        PlayerChat {
            global_index: 0,
            sender: Uuid::from_u128(7),
            index: 0,
            signature: None,
            body: SignedMessageBody {
                content: "hello".to_string(),
                timestamp_millis: 1,
                salt: 2,
                last_seen: Vec::new(),
            },
            unsigned_content: Some("hello".to_string()),
            unsigned_content_styled: Some(Vec::new()),
            unsigned_content_component,
            filter_mask: FilterMask {
                kind: FilterMaskKind::PassThrough,
                mask_words: Vec::new(),
            },
            chat_type: plain_chat_type(),
        }
    }

    fn command_suggestion(
        text: &str,
        tooltip_component: Option<NbtSummaryValue>,
    ) -> CommandSuggestion {
        CommandSuggestion {
            text: text.to_string(),
            tooltip: tooltip_component.as_ref().map(|_| text.to_string()),
            tooltip_styled: tooltip_component.as_ref().map(|_| Vec::new()),
            tooltip_component,
        }
    }

    fn advancement(id: &str, title: RichTextComponent) -> AdvancementSummary {
        AdvancementSummary {
            id: id.to_string(),
            parent: None,
            display: Some(AdvancementDisplaySummary {
                title,
                description: RichTextComponent::plain("description"),
                icon: AdvancementIconSummary {
                    item_id: 1,
                    count: 1,
                    component_patch: DataComponentPatchSummary::default(),
                },
                frame_type: AdvancementFrameType::Task,
                show_toast: false,
                hidden: false,
                background: None,
                x: 0.0,
                y: 0.0,
            }),
            requirements: Vec::new(),
            sends_telemetry_event: false,
        }
    }

    fn player_info_entry(display_name: RichTextComponent) -> PlayerInfoEntry {
        PlayerInfoEntry {
            profile_id: Uuid::from_u128(9),
            profile: None,
            listed: false,
            latency: 0,
            game_mode: GameType::default(),
            display_name: Some(display_name),
            show_hat: false,
            list_order: 0,
            chat_session: None,
        }
    }

    fn map_decoration(name: Option<RichTextComponent>) -> MapDecoration {
        MapDecoration {
            type_id: 0,
            x: 0,
            y: 0,
            rot: 0,
            name,
        }
    }

    fn team_parameters(player_suffix: RichTextComponent) -> PlayerTeamParameters {
        PlayerTeamParameters {
            display_name: RichTextComponent::plain("Red"),
            options: 0,
            nametag_visibility: TeamVisibility::Always,
            collision_rule: TeamCollisionRule::Always,
            color: ChatFormatting::Red,
            player_prefix: RichTextComponent::plain("[R] "),
            player_suffix,
        }
    }

    fn assert_packet_component_diagnostic(store: &WorldStore, message: &str) {
        assert!(store
            .apply_diagnostics()
            .last_apply_error
            .as_ref()
            .is_some_and(
                |error| error.source == "packet_component" && error.message.contains(message)
            ));
    }
}
