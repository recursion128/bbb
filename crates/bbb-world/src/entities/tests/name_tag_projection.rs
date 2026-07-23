use super::*;

use bbb_protocol::{
    packets::{
        NumberFormatSummary, ObjectiveRenderType, ScoreboardDisplaySlot, SetDisplayObjective,
        SetObjective, SetObjectiveMethod, SetObjectiveParameters, SetScore,
    },
    ComponentStyle, NbtSummaryEntry, NbtSummaryValue, RichTextComponent,
};

fn rich_text(text: &str, color: &str, bold: bool) -> RichTextComponent {
    RichTextComponent::try_from_component_nbt(NbtSummaryValue::Compound(vec![
        NbtSummaryEntry {
            name: "text".to_string(),
            value: NbtSummaryValue::String(text.to_string()),
        },
        NbtSummaryEntry {
            name: "color".to_string(),
            value: NbtSummaryValue::String(color.to_string()),
        },
        NbtSummaryEntry {
            name: "bold".to_string(),
            value: NbtSummaryValue::Byte(i8::from(bold)),
        },
    ]))
    .unwrap()
}

fn optional_component_data(
    data_id: u8,
    component: Option<RichTextComponent>,
) -> ProtocolEntityDataValue {
    ProtocolEntityDataValue {
        data_id,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_COMPONENT_ID,
        value: EntityDataValueKind::OptionalComponent(component),
    }
}

fn shared_flags_data(flags: i8) -> ProtocolEntityDataValue {
    let accessors = vanilla_entity_metadata_entity_accessors().unwrap();
    ProtocolEntityDataValue {
        data_id: accessors.shared_flags_data_id,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
        value: EntityDataValueKind::Byte(flags),
    }
}

fn item_frame_item_data(entity_type_id: i32, item: ItemStackSummary) -> ProtocolEntityDataValue {
    let accessors =
        vanilla_entity_metadata_item_frame_accessors_for_entity_type(entity_type_id).unwrap();
    ProtocolEntityDataValue {
        data_id: accessors.item_data_id,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
        value: EntityDataValueKind::ItemStack(item),
    }
}

fn assert_close(actual: f32, expected: f32, label: &str) {
    assert!(
        (actual - expected).abs() <= 1.0e-5,
        "{label}: expected {expected}, got {actual}"
    );
}

fn name_source(store: &WorldStore, entity_id: i32) -> EntityNameTagRenderSourceState {
    store
        .entity_name_tag_render_sources_at_partial_tick(1.0)
        .into_iter()
        .find(|source| source.entity_id == entity_id)
        .unwrap_or_else(|| panic!("missing name-tag source {entity_id}"))
}

fn rich_segment_text(segment: &EntityNameTagTextSegmentState) -> Option<&str> {
    match segment {
        EntityNameTagTextSegmentState::Rich(component) => Some(component.text()),
        _ => None,
    }
}

#[test]
fn entity_name_tag_source_preserves_rich_team_and_below_name_components() {
    let mut store = WorldStore::new();
    let entity_accessors = vanilla_entity_metadata_entity_accessors().unwrap();
    let chicken_uuid = Uuid::from_u128(0x1111_1111_2222_2222_3333_3333_4444_4444);
    let mut chicken = protocol_add_entity_with_type(10, VANILLA_ENTITY_TYPE_CHICKEN_ID);
    chicken.uuid = chicken_uuid;
    store.apply_add_entity(chicken);
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 10,
        values: vec![
            optional_component_data(
                entity_accessors.custom_name_data_id,
                Some(rich_text("Clucky", "yellow", true)),
            ),
            protocol_bool_data(entity_accessors.custom_name_visible_data_id, true),
            shared_flags_data(
                entity_accessors.shift_key_down_mask | entity_accessors.invisible_mask,
            ),
        ],
    }));
    assert!(store.apply_set_player_team(ProtocolSetPlayerTeam {
        name: "green".to_string(),
        method: ProtocolPlayerTeamMethod::Add.into(),
        parameters: Some(ProtocolPlayerTeamParameters {
            display_name: RichTextComponent::plain("Green"),
            options: 0,
            nametag_visibility: ProtocolTeamVisibility::Always,
            collision_rule: ProtocolTeamCollisionRule::Always,
            color: ProtocolChatFormatting::Green,
            player_prefix: rich_text("[", "red", false),
            player_suffix: rich_text("]", "aqua", false),
        }),
        players: vec![chicken_uuid.to_string()],
    }));

    assert!(store.apply_set_objective(SetObjective {
        objective_name: "health".to_string(),
        method: SetObjectiveMethod::Add.into(),
        parameters: Some(SetObjectiveParameters {
            display_name: rich_text("Health", "red", true),
            render_type: ObjectiveRenderType::Integer,
            number_format: Some(NumberFormatSummary::Styled(ComponentStyle {
                color: Some(0x55_FF_55),
                ..ComponentStyle::default()
            })),
        }),
    }));
    assert!(store.apply_set_display_objective(SetDisplayObjective {
        slot: ScoreboardDisplaySlot::BelowName,
        objective_name: Some("health".to_string()),
    }));
    assert!(store.apply_set_score(SetScore {
        owner: chicken_uuid.to_string(),
        objective_name: "health".to_string(),
        score: 17,
        display: None,
        number_format: Some(NumberFormatSummary::Fixed(rich_text("MAX", "aqua", false,))),
    }));

    let source = name_source(&store, 10);
    assert_eq!(source.name.segments.len(), 3);
    assert_eq!(rich_segment_text(&source.name.segments[0]), Some("["));
    assert_eq!(rich_segment_text(&source.name.segments[1]), Some("Clucky"));
    assert_eq!(rich_segment_text(&source.name.segments[2]), Some("]"));
    assert_eq!(source.name.inherited_style.color, Some(0x55_FF_55));
    let below = source.below_name.expect("below-name objective");
    assert_eq!(below.segments.len(), 3);
    assert_eq!(rich_segment_text(&below.segments[0]), Some("MAX"));
    assert!(matches!(
        &below.segments[1],
        EntityNameTagTextSegmentState::Styled(runs) if runs[0].text == " "
    ));
    assert_eq!(rich_segment_text(&below.segments[2]), Some("Health"));
    assert_close(source.attachment_y, 0.7, "chicken name attachment");
    assert_eq!(
        source.visibility,
        EntityNameTagVisibilityPolicy::MobOrAvatar {
            living: EntityNameTagLivingVisibilityState::TeamOverride { visible: false },
            entity_should_show_name: true,
            has_custom_name: true,
            is_discrete: true,
        }
    );
}

#[test]
fn entity_name_tag_source_drops_score_without_a_name_attachment() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        14,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    assert!(store.apply_set_objective(SetObjective {
        objective_name: "nearby".to_string(),
        method: SetObjectiveMethod::Add.into(),
        parameters: Some(SetObjectiveParameters {
            display_name: RichTextComponent::plain("Nearby"),
            render_type: ObjectiveRenderType::Integer,
            number_format: None,
        }),
    }));
    assert!(store.apply_set_display_objective(SetDisplayObjective {
        slot: ScoreboardDisplaySlot::BelowName,
        objective_name: Some("nearby".to_string()),
    }));

    assert!(store
        .entity_name_tag_render_sources_at_partial_tick(1.0)
        .iter()
        .all(|source| source.entity_id != 14));
}

#[test]
fn entity_name_tag_living_visibility_preserves_team_early_returns_and_vehicle_facts() {
    let mut store = WorldStore::new();
    let local_uuid = Uuid::from_u128(0xAAAA_AAAA_AAAA_AAAA_AAAA_AAAA_AAAA_AAAA);
    let target_uuid = Uuid::from_u128(0xBBBB_BBBB_BBBB_BBBB_BBBB_BBBB_BBBB_BBBB);
    store.apply_login(&protocol_play_login(20));
    let mut local = protocol_add_entity_with_type(20, VANILLA_ENTITY_TYPE_PLAYER_ID);
    local.uuid = local_uuid;
    store.apply_add_entity(local);
    store.apply_player_info_update(ProtocolPlayerInfoUpdate {
        actions: vec![ProtocolPlayerInfoAction::AddPlayer],
        entries: vec![protocol_player_info_entry_with_mode(
            local_uuid,
            ProtocolGameType::Survival,
        )],
    });
    let mut target = protocol_add_entity_with_type(21, VANILLA_ENTITY_TYPE_CHICKEN_ID);
    target.uuid = target_uuid;
    store.apply_add_entity(target);
    let accessors = vanilla_entity_metadata_entity_accessors().unwrap();
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 21,
        values: vec![protocol_bool_data(
            accessors.custom_name_visible_data_id,
            true,
        )],
    }));

    assert!(store.apply_set_player_team(ProtocolSetPlayerTeam {
        name: "target".to_string(),
        method: ProtocolPlayerTeamMethod::Add.into(),
        parameters: Some(ProtocolPlayerTeamParameters {
            display_name: RichTextComponent::plain("Target"),
            options: 0,
            nametag_visibility: ProtocolTeamVisibility::HideForOtherTeams,
            collision_rule: ProtocolTeamCollisionRule::Always,
            color: ProtocolChatFormatting::Reset,
            player_prefix: RichTextComponent::plain(""),
            player_suffix: RichTextComponent::plain(""),
        }),
        players: vec![target_uuid.to_string()],
    }));
    assert!(store.apply_set_player_team(ProtocolSetPlayerTeam {
        name: "local".to_string(),
        method: ProtocolPlayerTeamMethod::Add.into(),
        parameters: Some(ProtocolPlayerTeamParameters {
            display_name: RichTextComponent::plain("Local"),
            options: 0,
            nametag_visibility: ProtocolTeamVisibility::Always,
            collision_rule: ProtocolTeamCollisionRule::Always,
            color: ProtocolChatFormatting::Reset,
            player_prefix: RichTextComponent::plain(""),
            player_suffix: RichTextComponent::plain(""),
        }),
        players: vec!["PickTarget".to_string()],
    }));
    assert!(matches!(
        name_source(&store, 21).visibility,
        EntityNameTagVisibilityPolicy::MobOrAvatar {
            living: EntityNameTagLivingVisibilityState::TeamOverride { visible: false },
            ..
        }
    ));

    assert!(store.apply_set_player_team(ProtocolSetPlayerTeam {
        name: "target".to_string(),
        method: ProtocolPlayerTeamMethod::Join.into(),
        parameters: None,
        players: vec!["PickTarget".to_string()],
    }));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 21,
        values: vec![shared_flags_data(accessors.invisible_mask)],
    }));
    assert!(store.apply_set_player_team(ProtocolSetPlayerTeam {
        name: "target".to_string(),
        method: ProtocolPlayerTeamMethod::Change.into(),
        parameters: Some(ProtocolPlayerTeamParameters {
            display_name: RichTextComponent::plain("Target"),
            options: 2,
            nametag_visibility: ProtocolTeamVisibility::HideForOtherTeams,
            collision_rule: ProtocolTeamCollisionRule::Always,
            color: ProtocolChatFormatting::Reset,
            player_prefix: RichTextComponent::plain(""),
            player_suffix: RichTextComponent::plain(""),
        }),
        players: Vec::new(),
    }));
    assert!(matches!(
        name_source(&store, 21).visibility,
        EntityNameTagVisibilityPolicy::MobOrAvatar {
            living: EntityNameTagLivingVisibilityState::TeamOverride { visible: true },
            ..
        }
    ));

    let vehicle_uuid = Uuid::from_u128(0xCCCC_CCCC_CCCC_CCCC_CCCC_CCCC_CCCC_CCCC);
    let mut vehicle = protocol_add_entity_with_type(22, VANILLA_ENTITY_TYPE_CHICKEN_ID);
    vehicle.uuid = vehicle_uuid;
    store.apply_add_entity(vehicle);
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 22,
        values: vec![protocol_bool_data(
            accessors.custom_name_visible_data_id,
            true,
        )],
    }));
    store.apply_add_entity(protocol_add_entity_with_type(
        23,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 22,
        passenger_ids: vec![23],
    }));
    assert!(matches!(
        name_source(&store, 22).visibility,
        EntityNameTagVisibilityPolicy::MobOrAvatar {
            living: EntityNameTagLivingVisibilityState::General {
                visible_to_local_player: true,
                is_vehicle: true,
            },
            ..
        }
    ));
}

#[test]
fn entity_name_tag_renderer_policies_and_attachments_follow_vanilla_overrides() {
    let mut store = WorldStore::new();
    let accessors = vanilla_entity_metadata_entity_accessors().unwrap();
    for (id, entity_type_id) in [
        (30, VANILLA_ENTITY_TYPE_ARMOR_STAND_ID),
        (31, VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID),
        (32, VANILLA_ENTITY_TYPE_SNIFFER_ID),
        (33, VANILLA_ENTITY_TYPE_MANNEQUIN_ID),
        (34, VANILLA_ENTITY_TYPE_ITEM_FRAME_ID),
    ] {
        store.apply_add_entity(protocol_add_entity_with_type(id, entity_type_id));
    }
    for id in [30, 31, 32] {
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id,
            values: vec![protocol_bool_data(
                accessors.custom_name_visible_data_id,
                true,
            )],
        }));
    }
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 33,
        values: vec![optional_component_data(
            accessors.custom_name_data_id,
            Some(RichTextComponent::plain("Training Dummy")),
        )],
    }));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 34,
        values: vec![optional_component_data(
            accessors.custom_name_data_id,
            Some(RichTextComponent::plain("Frame Entity Name")),
        )],
    }));

    let armor_stand = name_source(&store, 30);
    assert_eq!(
        armor_stand.visibility,
        EntityNameTagVisibilityPolicy::ArmorStand {
            custom_name_visible: true,
        }
    );
    assert!(matches!(
        &armor_stand.name.segments[0],
        EntityNameTagTextSegmentState::Translation { key }
            if key == "entity.minecraft.armor_stand"
    ));
    assert!(matches!(
        name_source(&store, 31).visibility,
        EntityNameTagVisibilityPolicy::Entity {
            entity_should_show_name: true,
            has_custom_name: false,
        }
    ));

    let standing_sniffer = name_source(&store, 32);
    assert_close(
        standing_sniffer.attachment_y,
        2.05,
        "standing sniffer explicit attachment",
    );
    let (state_data_id, _) = vanilla_entity_metadata_sniffer_state_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_SNIFFER_ID,
    )
    .unwrap();
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 32,
        values: vec![protocol_enum_data(
            state_data_id,
            EntityDataEnumSerializer::SnifferState,
            VANILLA_SNIFFER_STATE_DIGGING_ID,
        )],
    }));
    let digging_sniffer = name_source(&store, 32);
    assert_close(
        digging_sniffer.attachment_y,
        1.35,
        "digging sniffer fallback attachment",
    );

    let mannequin = name_source(&store, 33);
    assert!(matches!(
        mannequin.visibility,
        EntityNameTagVisibilityPolicy::MobOrAvatar {
            entity_should_show_name: false,
            has_custom_name: true,
            ..
        }
    ));
    assert!(matches!(
        mannequin.below_name.as_ref().unwrap().segments[0],
        EntityNameTagTextSegmentState::Translation { ref key }
            if key == "entity.minecraft.mannequin.label"
    ));
    assert!(store
        .entity_name_tag_render_sources_at_partial_tick(1.0)
        .iter()
        .all(|source| source.entity_id != 34));
}

#[test]
fn item_frame_name_tag_source_uses_the_displayed_stacks_custom_name() {
    let mut store = WorldStore::new();
    let frame_uuid = Uuid::from_u128(0xEEEE_EEEE_EEEE_EEEE_EEEE_EEEE_EEEE_EEEE);
    let mut frame = protocol_add_entity_with_type(35, VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID);
    frame.uuid = frame_uuid;
    frame.data = DIRECTION_EAST;
    store.apply_add_entity(frame);

    let item_name = rich_text("Framed Relic", "light_purple", true);
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 35,
        values: vec![item_frame_item_data(
            VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID,
            ItemStackSummary {
                item_id: Some(1),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    added: 1,
                    added_type_ids: vec![
                        bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
                    ],
                    custom_name: Some(item_name.text().to_string()),
                    custom_name_styled: Some(item_name.styled().to_vec()),
                    custom_name_nbt: Some(item_name.component().clone()),
                    ..DataComponentPatchSummary::default()
                },
            },
        )],
    }));

    let source = name_source(&store, 35);
    assert_eq!(source.attachment_y, 0.5);
    assert_eq!(source.line_offset, 0);
    assert!(matches!(
        source.visibility,
        EntityNameTagVisibilityPolicy::ItemFrame {
            facing: ItemFrameFacing::East
        }
    ));
    assert_eq!(source.name.inherited_style, ComponentStyle::default());
    assert_eq!(source.name.segments.len(), 1);
    assert_eq!(
        rich_segment_text(&source.name.segments[0]),
        Some("Framed Relic")
    );
}

#[test]
fn entity_name_tag_player_uses_profile_name_and_deadmau5_line_offset() {
    let mut store = WorldStore::new();
    let player_uuid = Uuid::from_u128(0xDDDD_DDDD_DDDD_DDDD_DDDD_DDDD_DDDD_DDDD);
    let mut player = protocol_add_entity_with_type(40, VANILLA_ENTITY_TYPE_PLAYER_ID);
    player.uuid = player_uuid;
    store.apply_add_entity(player);
    let mut info = protocol_player_info_entry_with_mode(player_uuid, ProtocolGameType::Survival);
    info.profile.as_mut().unwrap().name = "deadmau5".to_string();
    store.apply_player_info_update(ProtocolPlayerInfoUpdate {
        actions: vec![ProtocolPlayerInfoAction::AddPlayer],
        entries: vec![info],
    });

    let source = name_source(&store, 40);
    assert_eq!(source.line_offset, -10);
    assert!(matches!(
        &source.name.segments[0],
        EntityNameTagTextSegmentState::Styled(runs) if runs[0].text == "deadmau5"
    ));
    assert!(matches!(
        source.visibility,
        EntityNameTagVisibilityPolicy::MobOrAvatar {
            entity_should_show_name: true,
            has_custom_name: false,
            ..
        }
    ));
}
