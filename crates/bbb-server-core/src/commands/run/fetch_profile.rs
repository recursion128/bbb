//! `/fetchprofile entity`.

use bbb_protocol::nbt_summary::entry;
use bbb_protocol::{
    canonical_snbt_string, chat_formattings::ChatFormatting, component_appended_nbt,
    component_translatable_nbt, validate_component_nbt, ComponentClickEvent, ComponentStyle,
    NbtSummaryEntry, NbtSummaryValue,
};

use crate::commands::parse::ParseResult;
use crate::commands::{chat_formatting_style, translatable, CommandSource};

use super::{
    single_damage_entity_argument, CommandCx, CommandRun, FetchProfileResult, FetchProfileTarget,
    Outcome,
};
use crate::commands::types::CommandDamageEntityRef;

/// `FetchProfileCommand.register` (`FetchProfileCommand.java:31-57`) through
/// `printForEntity` (`:153-164`), which answers 1 for every avatar.
pub(super) fn fetch_profile_entity(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
) -> Outcome {
    let target = cx.world.with_entity_reads(|reads| {
        let at = &cx.origin;
        Ok::<_, NbtSummaryValue>(
            match single_damage_entity_argument(parse, "entity", source, at, reads)? {
                CommandDamageEntityRef::Player(player) => {
                    FetchProfileTarget::Player { uuid: player.id }
                }
                CommandDamageEntityRef::Stored(stored) => FetchProfileTarget::Stored {
                    dimension: stored.dimension.clone(),
                    entity_id: stored.entity.id(),
                    uuid: stored.entity.uuid(),
                },
            },
        )
    })?;
    match cx.world.fetch_profile(&target) {
        FetchProfileResult::Avatar {
            profile,
            display_name,
        } => match avatar_profile_feedback(&profile, display_name) {
            Ok(message) => run.succeed(message, false),
            Err(error) => run.fail(translatable(
                "commands.fetchprofile.failed_to_serialize",
                vec![NbtSummaryValue::string(error)],
            )),
        },
        FetchProfileResult::NotAvatar { display_name } => {
            return Err(translatable(
                "commands.fetchprofile.no_profile",
                vec![display_name],
            ));
        }
        // A mannequin whose profile does not resolve reports nothing.
        FetchProfileResult::Unavailable => {}
    }
    Ok(1)
}

/// `FetchProfileCommand.reportResolvedProfile` (`FetchProfileCommand.java:59-113`).
pub(crate) fn avatar_profile_feedback(
    profile: &NbtSummaryValue,
    display_name: NbtSummaryValue,
) -> Result<NbtSummaryValue, String> {
    validate_profile(profile)?;
    validate_component_nbt(&display_name, "fetchprofile display name")?;

    let profile_snbt = canonical_snbt_string(profile);
    let sprite = player_sprite(profile, false);
    validate_component_nbt(&sprite, "fetchprofile player sprite")?;
    let sprite_snbt = canonical_snbt_string(&sprite);
    let white_sprite = player_sprite(profile, true);
    validate_component_nbt(&white_sprite, "fetchprofile white player sprite")?;

    let entries = vec![
        clickable(
            "commands.fetchprofile.copy_component",
            Vec::new(),
            ComponentClickEvent::CopyToClipboard {
                value: profile_snbt.clone().into(),
            },
        )?,
        clickable(
            "commands.fetchprofile.give_item",
            Vec::new(),
            ComponentClickEvent::RunCommand {
                command: format!("give @s minecraft:player_head[profile={profile_snbt}]").into(),
            },
        )?,
        clickable(
            "commands.fetchprofile.summon_mannequin",
            Vec::new(),
            ComponentClickEvent::RunCommand {
                command: format!("summon minecraft:mannequin ~ ~ ~ {{profile:{profile_snbt}}}")
                    .into(),
            },
        )?,
        clickable(
            "commands.fetchprofile.copy_text",
            vec![white_sprite],
            ComponentClickEvent::CopyToClipboard {
                value: sprite_snbt.into(),
            },
        )?,
    ];
    let mut list = Vec::with_capacity(7);
    for entry in entries {
        if !list.is_empty() {
            list.push(NbtSummaryValue::string(" "));
        }
        list.push(entry);
    }
    let clickable = component_appended_nbt(&ComponentStyle::default(), list)?;
    let message = translatable(
        "commands.fetchprofile.entity.success",
        vec![display_name, clickable],
    );
    validate_component_nbt(&message, "fetchprofile success")?;
    Ok(message)
}

fn clickable(
    key: &str,
    arguments: Vec<NbtSummaryValue>,
    click_event: ComponentClickEvent,
) -> Result<NbtSummaryValue, String> {
    let inner = component_translatable_nbt(
        key,
        arguments,
        &ComponentStyle {
            click_event: Some(click_event),
            ..chat_formatting_style(ChatFormatting::Green)
        },
    )?;
    Ok(translatable("chat.square_brackets", vec![inner]))
}

fn player_sprite(profile: &NbtSummaryValue, white: bool) -> NbtSummaryValue {
    let mut entries = vec![entry("player", profile.clone())];
    if white {
        entries.push(entry("color", NbtSummaryValue::string("white")));
    }
    NbtSummaryValue::Compound(entries)
}

fn validate_profile(profile: &NbtSummaryValue) -> Result<(), String> {
    let NbtSummaryValue::Compound(entries) = profile else {
        return Err("profile is not a compound".to_string());
    };
    let mut seen = Vec::with_capacity(entries.len());
    for entry in entries {
        if seen.contains(&entry.name) {
            return Err(format!(
                "profile field {} is duplicated",
                entry.name.to_display_string()
            ));
        }
        seen.push(entry.name.clone());
        // The canonical profile writer names its fields in ASCII; that codec owns the bound.
        match (
            [
                "name",
                "id",
                "properties",
                "texture",
                "cape",
                "elytra",
                "model",
            ]
            .into_iter()
            .find(|name| entry.name == *name)
            .unwrap_or(""),
            &entry.value,
        ) {
            ("name", NbtSummaryValue::String(name)) => validate_player_name(name)?,
            ("id", NbtSummaryValue::IntArray(values)) if values.len() == 4 => {}
            ("properties", NbtSummaryValue::List(properties)) => validate_properties(properties)?,
            ("texture" | "cape" | "elytra", NbtSummaryValue::String(id))
                if id
                    .try_to_string()
                    .ok()
                    .and_then(|id| bbb_protocol::normalize_resource_location(id).ok())
                    .is_some_and(|parsed| id == &parsed) => {}
            ("model", NbtSummaryValue::String(model)) if (model == "slim" || model == "wide") => {}
            ("name" | "id" | "properties" | "texture" | "cape" | "elytra" | "model", _) => {
                return Err(format!(
                    "profile field {} has an invalid value",
                    entry.name.to_display_string()
                ));
            }
            _ => {
                return Err(format!(
                    "unexpected profile field {}",
                    entry.name.to_display_string()
                ))
            }
        }
    }
    Ok(())
}

fn validate_player_name(name: &bbb_protocol::JavaString) -> Result<(), String> {
    if name
        .try_to_string()
        .is_ok_and(|name| bbb_protocol::player_names::valid_player_name(&name))
    {
        Ok(())
    } else {
        Err("profile player name is invalid".into())
    }
}

fn validate_properties(properties: &[NbtSummaryValue]) -> Result<(), String> {
    if properties.is_empty() || properties.len() > 16 {
        return Err("profile properties must contain 1..=16 entries".to_string());
    }
    for property in properties {
        let NbtSummaryValue::Compound(entries) = property else {
            return Err("profile property is not a compound".to_string());
        };
        let mut seen = Vec::with_capacity(entries.len());
        for entry in entries {
            if !["name", "value", "signature"]
                .iter()
                .any(|name| entry.name == *name)
            {
                return Err(format!(
                    "unexpected profile property field {}",
                    entry.name.to_display_string()
                ));
            }
            if seen.contains(&entry.name) {
                return Err(format!(
                    "profile property field {} is duplicated",
                    entry.name.to_display_string()
                ));
            }
            seen.push(entry.name.clone());
        }
        let string = |name: &str, required: bool, limit: usize| {
            let value = entries.iter().find(|entry| entry.name == name);
            match value {
                Some(NbtSummaryEntry {
                    value: NbtSummaryValue::String(value),
                    ..
                }) if value.units().len() <= limit => Ok(()),
                None if !required => Ok(()),
                _ => Err(format!("profile property field {name} is invalid")),
            }
        };
        string("name", true, 64)?;
        string("value", true, 32_767)?;
        string("signature", false, 1_024)?;
    }
    Ok(())
}
