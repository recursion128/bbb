//! Shared command component and message resolution.

pub(super) mod nbt;

use bbb_protocol::component::{
    component_bool, component_record_type, COMPONENT_CONTENT_FIELDS, COMPONENT_DATA_SOURCE_FIELDS,
};
use bbb_protocol::nbt_summary::value_field as field;
use bbb_protocol::{
    component_appended_nbt, component_literal_nbt, component_node_style_from_nbt_value,
    component_styled_literal_nbt, component_with_siblings_nbt,
    parse_command_nbt_path_value_prefix_utf16, ComponentHoverEvent, ComponentStyle,
    NbtSummaryEntry, NbtSummaryValue,
};
use bbb_server_persistence::Java26ScoreboardNumberFormat;

use crate::commands::parse::{read_block_pos, ArgumentValue, ParseResult};
use crate::commands::types::CommandDamageEntityRef;
use crate::commands::types::CommandPlayer;
use crate::commands::{translatable, CommandOrigin, CommandPermission, CommandSource};
use crate::world::EntityReads;

type Resolution = Result<NbtSummaryValue, NbtSummaryValue>;

pub(crate) fn resolve_loot_text(
    value: &NbtSummaryValue,
    source: &CommandSource,
    at: &CommandOrigin,
    world: &EntityReads<'_>,
) -> Resolution {
    let Some(entity) = super::source_entity_state(source, world) else {
        return Ok(value.clone());
    };
    resolve_entity_text(value, entity, &at.level, world)
}

/// `ComponentUtils.resolve` against
/// `Entity.createCommandSourceStackForNameResolution` (`Entity.java:3556-3568`)
/// with gamemaster permission, as `SetNameFunction` (`:69-85`) and
/// `TextDisplay.readAdditionalSaveData` (`Display.java:773-786`) build it.
pub(crate) fn resolve_entity_text(
    value: &NbtSummaryValue,
    entity: CommandDamageEntityRef<'_>,
    level: &bbb_server_types::save::Java26LevelKey,
    world: &EntityReads<'_>,
) -> Resolution {
    let source = super::command_entity_source(entity, CommandPermission::GameMaster, world);
    let at = CommandOrigin {
        position: entity.position(),
        rotation: entity.rotation(),
        level: level.clone(),
        anchor: bbb_protocol::entity_anchors::EntityAnchor::Feet,
        eye_height: Some(entity.eye_height()),
    };
    resolve(value, &source, &at, world, None)
}

// MessageArgument.java:92-119, :167-170.
pub(super) fn message(
    parse: &ParseResult,
    name: &str,
    source: &CommandSource,
    at: &CommandOrigin,
    world: &EntityReads<'_>,
) -> Resolution {
    let Some(ArgumentValue::Message(message)) = parse.argument(name) else {
        unreachable!("message argument");
    };
    if message.selectors.is_empty() || source.permission < CommandPermission::GameMaster {
        return Ok(component_literal_nbt(&message.text));
    }
    let mut children = Vec::new();
    let mut cursor = 0;
    for (start, end, selector) in &message.selectors {
        if cursor < *start {
            children.push(component_literal_nbt(&message.text.units()[cursor..*start]));
        }
        let names = super::selector::select_damage_entity_states(selector, source, at, world)?
            .into_iter()
            .map(|entity| display_name(entity, world))
            .collect::<Result<Vec<_>, _>>()?;
        children.push(join(
            names,
            super::chat_formatting_style(bbb_protocol::chat_formattings::ChatFormatting::Gray),
        ));
        cursor = *end;
    }
    if cursor < message.text.units().len() {
        children.push(component_literal_nbt(&message.text.units()[cursor..]));
    }
    // The prefix is the literal root, even when it is empty.
    let root = component_literal_nbt(&message.text.units()[..message.selectors[0].0]);
    if message.selectors[0].0 != 0 {
        children.remove(0);
    }
    Ok(component_with_siblings_nbt(root, children)
        .expect("resolved message children are components"))
}

pub(crate) fn resolve(
    value: &NbtSummaryValue,
    source: &CommandSource,
    at: &CommandOrigin,
    world: &EntityReads<'_>,
    recipient: Option<CommandPlayer<'_>>,
) -> Resolution {
    bbb_protocol::validate_component_nbt_with_registry(
        value,
        "component",
        world.content_registry(),
    )
    .map_err(component_literal_nbt)?;
    resolve_at(value, source, at, world, recipient, 0)
}

// ComponentUtils.java:54-80; ResolutionContext.java:28-33.
fn resolve_at(
    value: &NbtSummaryValue,
    source: &CommandSource,
    at: &CommandOrigin,
    world: &EntityReads<'_>,
    recipient: Option<CommandPlayer<'_>>,
    depth: usize,
) -> Resolution {
    if depth > 100 {
        return Ok(value.clone());
    }
    let fields = match value {
        NbtSummaryValue::Compound(fields) => fields,
        NbtSummaryValue::List(values) => {
            let mut values = values.iter();
            let head = resolve_at(
                values
                    .next()
                    .expect("validated component list is non-empty"),
                source,
                at,
                world,
                recipient,
                depth,
            )?;
            let siblings = values
                .map(|value| resolve_at(value, source, at, world, recipient, depth + 1))
                .collect::<Result<Vec<_>, _>>()?;
            return component_with_siblings_nbt(head, siblings).map_err(component_literal_nbt);
        }
        _ => return Ok(value.clone()),
    };
    let child_depth = depth + 1;
    let kind = component_record_type(fields, "type", &COMPONENT_CONTENT_FIELDS, "component")
        .expect("validated component content type");
    let mut result = if kind == "selector" {
        let selector = string_field(value, "selector").expect("validated selector");
        let (_, selector) = crate::commands::parse::read_selector_with_registry(
            selector.units(),
            0,
            source.permission,
            world.content_registry(),
        )
        .map_err(|error| error.message)?;
        let separator = separator(value, source, at, world, recipient, child_depth)?;
        let names = super::selector::select_damage_entity_states(&selector, source, at, world)?
            .into_iter()
            .map(|entity| display_name(entity, world))
            .collect::<Result<Vec<_>, _>>()?;
        join_with(names, separator)
    } else if kind == "score" {
        score_value(
            field(value, "score").expect("validated score"),
            source,
            at,
            world,
            recipient,
        )?
    } else if kind == "nbt" {
        nbt_value(value, fields, source, at, world, recipient, child_depth)?
    } else {
        let mut contents = fields
            .iter()
            .filter(|field| field.name != "extra")
            .cloned()
            .collect::<Vec<_>>();
        if kind == "translatable" {
            if let Some(arguments) = contents.iter_mut().find(|field| field.name == "with") {
                if let NbtSummaryValue::List(values) = &mut arguments.value {
                    for argument in values {
                        if matches!(
                            argument,
                            NbtSummaryValue::Compound(_) | NbtSummaryValue::List(_)
                        ) {
                            *argument =
                                resolve_at(argument, source, at, world, recipient, child_depth)?;
                        }
                    }
                }
            }
        } else if kind == "object" {
            if let Some(fallback) = contents.iter_mut().find(|field| field.name == "fallback") {
                fallback.value =
                    resolve_at(&fallback.value, source, at, world, recipient, child_depth)?;
            }
        }
        NbtSummaryValue::Compound(contents)
    };
    let mut siblings = Vec::new();
    if let Some(NbtSummaryValue::List(values)) = field(value, "extra") {
        for value in values {
            siblings.push(resolve_at(
                value,
                source,
                at,
                world,
                recipient,
                child_depth,
            )?);
        }
    }
    result = component_with_siblings_nbt(result, siblings).map_err(component_literal_nbt)?;
    let mut style = component_node_style_from_nbt_value(value);
    if let Some(ComponentHoverEvent::ShowText {
        component: Some(hover),
        ..
    }) = &style.hover_event
    {
        let resolved = crate::commands::component_with_registry(
            resolve_at(hover, source, at, world, recipient, child_depth)?,
            world.content_registry(),
        );
        style.hover_event = Some(ComponentHoverEvent::ShowText {
            value: resolved.styled().to_vec(),
            component: Some(resolved.component().clone()),
        });
    }
    Ok(bbb_protocol::collapsed_component_nbt(
        &super::scoreboard::with_style(result, style),
    ))
}

// ScoreContents.java:41-86; ReadOnlyScoreInfo.java:15-17.
fn score_value(
    score: &NbtSummaryValue,
    source: &CommandSource,
    at: &CommandOrigin,
    world: &EntityReads<'_>,
    recipient: Option<CommandPlayer<'_>>,
) -> Resolution {
    let name =
        bbb_protocol::JavaString::from_nbt(field(score, "name").expect("validated score name"))
            .expect("score name is a string");
    let objective = bbb_protocol::JavaString::from_nbt(
        field(score, "objective").expect("validated score objective"),
    )
    .expect("score objective is a string");
    let mut holder = name.clone();
    let selector = crate::commands::parse::read_selector_with_registry(
        name.units(),
        0,
        CommandPermission::Owner,
        world.content_registry(),
    )
    .ok()
    .filter(|(end, _)| *end == name.units().len())
    .map(|(_, selector)| selector);
    if let Some(selector) = selector {
        if name.units().first() == Some(&u16::from(b'@'))
            && source.permission < CommandPermission::GameMaster
        {
            return Err(translatable(
                "argument.entity.selector.not_allowed",
                Vec::new(),
            ));
        }
        let entities = super::selector::select_damage_entity_states(&selector, source, at, world)?;
        if entities.len() > 1 {
            return Err(translatable("argument.entity.toomany", Vec::new()));
        }
        if let Some(entity) = entities.first() {
            holder = entity.with_scoreboard_name(|name| name.into());
        }
    }
    if holder == "*" {
        if let Some(player) = recipient {
            holder = player.name.into();
        } else if let Some(entity) = super::source_entity_state(source, world) {
            holder = entity.with_scoreboard_name(|name| name.into());
        }
    }
    let board = world.scoreboard;
    let Some(objective) = board.objective(&objective) else {
        return Ok(component_literal_nbt(""));
    };
    let Some(score) = board.score(&holder, &objective.name) else {
        return Ok(component_literal_nbt(""));
    };
    let formatted = match score
        .number_format
        .as_ref()
        .or(objective.number_format.as_ref())
    {
        Some(Java26ScoreboardNumberFormat::Blank) => component_literal_nbt(""),
        Some(Java26ScoreboardNumberFormat::Fixed(value)) => value.clone(),
        Some(Java26ScoreboardNumberFormat::Styled(style)) => super::scoreboard::with_style(
            component_literal_nbt(score.value.to_string()),
            bbb_protocol::component_style_from_nbt_value(style)
                .expect("validated score number style"),
        ),
        None => component_literal_nbt(score.value.to_string()),
    };
    Ok(formatted)
}

// NbtContents.java:59-98; BlockDataSource.java:34-45; EntityDataSource.java:21-24; StorageDataSource.java:16-19.
fn nbt_value(
    value: &NbtSummaryValue,
    fields: &[NbtSummaryEntry],
    source: &CommandSource,
    at: &CommandOrigin,
    world: &EntityReads<'_>,
    recipient: Option<CommandPlayer<'_>>,
    depth: usize,
) -> Resolution {
    let path = string_field(value, "nbt").expect("validated NBT path");
    let (_, path) = parse_command_nbt_path_value_prefix_utf16(path.units())
        .map_err(|error| component_literal_nbt(error.to_string()))?;
    let mut roots = Vec::new();
    let data_source =
        component_record_type(fields, "source", &COMPONENT_DATA_SOURCE_FIELDS, "component")
            .expect("validated component data source");
    if data_source == "storage" {
        let id = string_field(value, "storage").expect("validated storage");
        let id = id
            .try_to_string()
            .map_err(|error| component_literal_nbt(error.to_string()))?;
        let id = if id.contains(':') {
            id
        } else {
            format!("minecraft:{id}")
        };
        roots.push(
            world
                .storage(&id)
                .unwrap_or_else(|| NbtSummaryValue::Compound(Vec::new())),
        );
    } else if data_source == "entity" {
        let selector = string_field(value, "entity").expect("validated entity selector");
        let (_, selector) = crate::commands::parse::read_selector_with_registry(
            selector.units(),
            0,
            source.permission,
            world.content_registry(),
        )
        .map_err(|error| error.message)?;
        for entity in super::selector::select_damage_entity_states(&selector, source, at, world)? {
            if let Some(data) =
                super::entity::command_entity_data(entity, world).map_err(component_literal_nbt)?
            {
                roots.push(data);
            }
        }
    } else if data_source == "block" {
        let coordinates = string_field(value, "block").expect("validated block coordinates");
        let (_, coordinates) =
            read_block_pos(coordinates.units(), 0).map_err(|error| error.message)?;
        let ArgumentValue::BlockPos(coordinates) = coordinates else {
            unreachable!("block coordinates");
        };
        let pos = coordinates.block_pos_from(at.position, at.rotation, at.anchored_position());
        if let Some(blocks) = world
            .blocks(&at.level)
            .filter(|blocks| blocks.has_chunk_at(pos))
        {
            if let Some(data) =
                super::blocks::block_entity_data_at(&blocks, pos, Some(world.resolver().as_dyn()))?
            {
                roots.push(data);
            }
        }
    }
    let separator = if let Some(separator) = field(value, "separator") {
        resolve_at(separator, source, at, world, recipient, depth)?
    } else {
        component_literal_nbt(", ")
    };
    // Codec booleans use the last entry; command payload reads retain their first-entry contract.
    let enabled = |name: &str| {
        fields
            .iter()
            .rev()
            .find(|entry| entry.name == name)
            .and_then(|entry| component_bool(&entry.value))
            == Some(true)
    };
    let interpreting = enabled("interpret");
    let plain = enabled("plain");
    let mut elements = Vec::new();
    for root in roots {
        for element in path.get(&root).unwrap_or_default() {
            if interpreting {
                if bbb_protocol::validate_component_nbt_with_registry(
                    &element,
                    "interpreted NBT component",
                    world.content_registry(),
                )
                .is_ok()
                {
                    if let Ok(resolved) = resolve_at(&element, source, at, world, recipient, depth)
                    {
                        elements.push(resolved);
                    }
                }
            } else {
                elements.push(nbt::display(&element, plain));
            }
        }
    }
    let mut iter = elements.into_iter();
    let mut result = iter.next().unwrap_or_else(|| component_literal_nbt(""));
    for element in iter {
        result = component_with_siblings_nbt(result, vec![separator.clone(), element])
            .expect("NBT values are components");
    }
    Ok(result)
}

fn separator(
    value: &NbtSummaryValue,
    source: &CommandSource,
    at: &CommandOrigin,
    world: &EntityReads<'_>,
    recipient: Option<CommandPlayer<'_>>,
    depth: usize,
) -> Resolution {
    match field(value, "separator") {
        Some(value) => resolve_at(value, source, at, world, recipient, depth),
        None => Ok(component_styled_literal_nbt(
            ", ",
            &super::chat_formatting_style(bbb_protocol::chat_formattings::ChatFormatting::Gray),
        )
        .expect("separator style")),
    }
}

fn join(names: Vec<NbtSummaryValue>, style: ComponentStyle) -> NbtSummaryValue {
    join_with(
        names,
        component_styled_literal_nbt(", ", &style).expect("separator style"),
    )
}

fn join_with(names: Vec<NbtSummaryValue>, separator: NbtSummaryValue) -> NbtSummaryValue {
    if names.len() == 1 {
        return names.into_iter().next().unwrap();
    }
    let mut children = Vec::new();
    for name in names {
        if !children.is_empty() {
            children.push(separator.clone());
        }
        children.push(name);
    }
    component_appended_nbt(&ComponentStyle::default(), children)
        .expect("entity names are components")
}

// Entity.java:3207-3210; Player.java:1676-1686; PlayerTeam.java:116-128.
pub(super) fn display_name(
    entity: CommandDamageEntityRef<'_>,
    world: &EntityReads<'_>,
) -> Resolution {
    Ok(match entity {
        CommandDamageEntityRef::Player(player) => crate::commands::team_player_display_name_nbt(
            &crate::commands::CommandPlayerView {
                name: player.name.into(),
                uuid: player.id,
            },
            world.scoreboard,
        ),
        CommandDamageEntityRef::Stored(stored) => crate::world::display_names::entity_display_name(
            stored.entity.data().entity_resource_id(),
            stored.entity.uuid(),
            super::selector::entity_name(entity, world)?,
            super::selector::team_name(entity, world).and_then(|name| world.scoreboard.team(name)),
        ),
    })
}

fn string_field<'a>(
    value: &'a NbtSummaryValue,
    name: &str,
) -> Option<&'a bbb_protocol::JavaString> {
    match field(value, name) {
        Some(NbtSummaryValue::String(value)) => Some(value),
        _ => None,
    }
}
