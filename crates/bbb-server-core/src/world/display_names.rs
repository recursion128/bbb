//! Entity and player display names with team formatting and root interactions.
use bbb_protocol::nbt_summary::entry as nbt_entry;
use bbb_protocol::packets::GameProfile;
use bbb_protocol::{
    component_appended_nbt, component_hover_uuid_nbt, component_literal_nbt,
    component_styled_literal_nbt, component_translatable_nbt, component_with_siblings_nbt,
    ComponentClickEvent, ComponentHoverEvent, ComponentStyle, NbtSummaryValue,
};
use uuid::Uuid;

use super::scoreboard::PlayerTeam;
use crate::entity::Entity;

/// `Player.getDisplayName` (`Player.java:1676-1685`) through
/// `PlayerTeam.formatNameForTeam` (`PlayerTeam.java:116-127`).
pub(super) fn player_display_name(
    profile: &GameProfile,
    team: Option<&PlayerTeam>,
) -> NbtSummaryValue {
    let name = component_literal_nbt(&profile.name);
    let display = format_name_for_team(name, team);
    apply_root_style(
        display,
        ComponentStyle {
            click_event: Some(ComponentClickEvent::SuggestCommand {
                command: format!("/tell {} ", profile.name).into(),
            }),
            hover_event: Some(entity_hover(
                "minecraft:player",
                profile.uuid,
                component_literal_nbt(&profile.name),
            )),
            insertion: Some(profile.name.clone().into()),
            ..ComponentStyle::default()
        },
    )
}

/// `Entity.getDisplayName` (`Entity.java:3207-3209,3358-3360`) through
/// `PlayerTeam.formatNameForTeam` (`PlayerTeam.java:116-127`).
pub(super) fn stored_display_name(entity: &Entity, team: Option<&PlayerTeam>) -> NbtSummaryValue {
    stored_display_name_parts(
        entity.data().entity_resource_id(),
        entity.uuid(),
        entity.custom_name(),
        team,
    )
}

pub(super) fn stored_display_name_parts(
    resource_id: &str,
    uuid: uuid::Uuid,
    custom_name: Option<&bbb_protocol::RichTextComponent>,
    team: Option<&PlayerTeam>,
) -> NbtSummaryValue {
    let name = entity_name(resource_id, custom_name);
    entity_display_name(resource_id, uuid, name, team)
}

pub(crate) fn entity_display_name(
    resource_id: &str,
    uuid: uuid::Uuid,
    name: NbtSummaryValue,
    team: Option<&PlayerTeam>,
) -> NbtSummaryValue {
    let display = format_name_for_team(name.clone(), team);
    apply_root_style(
        display,
        ComponentStyle {
            hover_event: Some(entity_hover(resource_id, uuid, name)),
            insertion: Some(uuid.to_string().into()),
            ..ComponentStyle::default()
        },
    )
}

/// `Entity.getName` removes click actions from the root and its siblings
/// (`Entity.java:2870-2884`).
fn entity_name(
    resource_id: &str,
    custom_name: Option<&bbb_protocol::RichTextComponent>,
) -> NbtSummaryValue {
    custom_name.map_or_else(
        || {
            let (namespace, path) = resource_id
                .split_once(':')
                .unwrap_or(("minecraft", resource_id));
            component_translatable_nbt(
                format!("entity.{namespace}.{}", path.replace('/', ".")),
                Vec::new(),
                &ComponentStyle::default(),
            )
            .expect("an entity description is a valid component")
        },
        |name| remove_sibling_click_actions(name.component()),
    )
}

pub(super) fn remove_sibling_click_actions(component: &NbtSummaryValue) -> NbtSummaryValue {
    match component {
        NbtSummaryValue::Compound(fields) => NbtSummaryValue::Compound(
            fields
                .iter()
                .filter(|entry| entry.name != "click_event")
                .map(|field| {
                    if field.name != "extra" {
                        return field.clone();
                    }
                    let value = match &field.value {
                        NbtSummaryValue::List(siblings) => NbtSummaryValue::List(
                            siblings.iter().map(remove_sibling_click_actions).collect(),
                        ),
                        value => value.clone(),
                    };
                    nbt_entry("extra", value)
                })
                .collect(),
        ),
        NbtSummaryValue::List(components) => NbtSummaryValue::List(
            components
                .iter()
                .map(remove_sibling_click_actions)
                .collect(),
        ),
        value => value.clone(),
    }
}

fn format_name_for_team(name: NbtSummaryValue, team: Option<&PlayerTeam>) -> NbtSummaryValue {
    let Some(team) = team else {
        return name;
    };
    let style = ComponentStyle::with_chat_formatting_color(team.color).unwrap_or_default();
    component_appended_nbt(
        &style,
        vec![
            team.player_prefix
                .clone()
                .unwrap_or_else(|| component_literal_nbt("")),
            name,
            team.player_suffix
                .clone()
                .unwrap_or_else(|| component_literal_nbt("")),
        ],
    )
    .expect("scoreboard team components are validated on ingress")
}

fn apply_root_style(component: NbtSummaryValue, style: ComponentStyle) -> NbtSummaryValue {
    let NbtSummaryValue::Compound(style_fields) =
        component_styled_literal_nbt("", &style).expect("owned display style is valid")
    else {
        unreachable!("a styled literal is a compound")
    };
    let style_fields = style_fields.into_iter().skip(1).collect::<Vec<_>>();
    let mut fields = match component {
        NbtSummaryValue::String(text) => vec![nbt_entry("text", NbtSummaryValue::string(text))],
        NbtSummaryValue::Compound(fields) => fields,
        NbtSummaryValue::List(children) => {
            let mut children = children.into_iter();
            let root = children.next().expect("a component list is nonempty");
            return component_with_siblings_nbt(apply_root_style(root, style), children.collect())
                .expect("a decorated component has a record root");
        }
        _ => unreachable!("entity names are validated components"),
    };
    fields.retain(|field| {
        !style_fields
            .iter()
            .any(|replacement| replacement.name == field.name)
    });
    fields.extend(style_fields);
    NbtSummaryValue::Compound(fields)
}

fn entity_hover(id: &str, uuid: Uuid, name: NbtSummaryValue) -> ComponentHoverEvent {
    ComponentHoverEvent::ShowEntity {
        id: id.to_owned(),
        uuid: component_hover_uuid_nbt(uuid),
        name: None,
        name_component: Some(name),
    }
}
