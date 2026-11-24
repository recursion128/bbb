use bbb_facts::boss_bar_styles::{BossBarColor, BossBarOverlay};
use bbb_protocol::{
    chat_formattings::ChatFormatting, component_translatable_nbt, ComponentStyle, NbtSummaryValue,
    RichTextComponent,
};
use rustc_hash::FxHashSet;

use crate::commands::execution::StoreTarget;
use crate::commands::{
    format_list, team_player_display_name_nbt, translatable, CommandBossBarMutation,
    CommandBossBarState, CommandOrigin, CommandPlayerView, CommandSource,
};
use crate::world::scoreboard::Scoreboard;
use crate::world::ServerWorld;

use super::{
    component_argument, identifier_argument, integer_argument, scoreboard::show_text,
    select_player_states, ArgumentValue, CommandCx, CommandRun, Outcome, ParseResult,
};

pub(super) fn bossbar(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
    path: &[&str],
) -> Outcome {
    // BossBarCommands.java:30-333.
    let read = |result: Outcome| result.map(|value| (value, None));
    let write = |result: Result<(i32, CommandBossBarMutation), NbtSummaryValue>| {
        result.map(|(value, mutation)| (value, Some(mutation)))
    };
    let (world, at) = (&mut *cx.world, &cx.origin);
    let (result, mutation) = match path {
        ["bossbar", "add", "id", "name"] => write(create(run, parse, source, world, at)),
        ["bossbar", "remove", "id"] => write(remove(run, parse, world)),
        ["bossbar", "list"] => read(Ok(list(run, world))),
        ["bossbar", "set", "id", "name", "name"] => write(set_name(run, parse, source, world, at)),
        ["bossbar", "set", "id", "color", color] => {
            write(set_color(run, parse, world, color_for_name(color)))
        }
        ["bossbar", "set", "id", "style", overlay] => {
            write(set_overlay(run, parse, world, overlay_for_name(overlay)))
        }
        ["bossbar", "set", "id", "value", "value"] => write(set_value(run, parse, world)),
        ["bossbar", "set", "id", "max", "max"] => write(set_max(run, parse, world)),
        ["bossbar", "set", "id", "visible", "visible"] => write(set_visible(run, parse, world)),
        ["bossbar", "set", "id", "players"] => {
            write(set_players(run, parse, source, world, at, false))
        }
        ["bossbar", "set", "id", "players", "targets"] => {
            write(set_players(run, parse, source, world, at, true))
        }
        ["bossbar", "get", "id", "value"] => read(get_value(run, parse, world)),
        ["bossbar", "get", "id", "max"] => read(get_max(run, parse, world)),
        ["bossbar", "get", "id", "visible"] => read(get_visible(run, parse, world)),
        ["bossbar", "get", "id", "players"] => read(get_players(run, parse, world)),
        _ => unreachable!("no bossbar executor for {path:?}"),
    }?;
    if let Some(mutation) = mutation {
        cx.world.mutate_boss_bar(mutation, cx.events);
    }
    Ok(result)
}

fn create(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
) -> Result<(i32, CommandBossBarMutation), NbtSummaryValue> {
    let id = identifier_argument(parse, "id");
    let resolved = world.with_entity_reads(|reads| {
        super::text::resolve(component_argument(parse, "name"), source, at, reads, None)
    })?;
    let name =
        crate::commands::component_with_registry(resolved, world.plugins().content_registry());
    if world.has_boss_bar(id) {
        return Err(translatable(
            "commands.bossbar.create.failed",
            vec![NbtSummaryValue::string(id)],
        ));
    }
    let mutation = CommandBossBarMutation::Create {
        id: id.to_string(),
        name: name.clone(),
        source_level: at.level.clone(),
    };
    run.succeed(
        translatable(
            "commands.bossbar.create.success",
            vec![display_name(id, &name, BossBarColor::White)],
        ),
        true,
    );
    Ok((
        i32::try_from(world.boss_bar_count() + 1).unwrap_or(i32::MAX),
        mutation,
    ))
}

fn remove(
    run: &mut CommandRun,
    parse: &ParseResult,
    world: &ServerWorld,
) -> Result<(i32, CommandBossBarMutation), NbtSummaryValue> {
    let event = event(parse, world)?;
    let mutation = CommandBossBarMutation::Remove {
        id: event.id.clone(),
    };
    run.succeed(
        translatable(
            "commands.bossbar.remove.success",
            vec![event_display_name(event)],
        ),
        true,
    );
    Ok((
        i32::try_from(world.boss_bar_count() - 1).unwrap_or(i32::MAX),
        mutation,
    ))
}

fn list(run: &mut CommandRun, world: &ServerWorld) -> i32 {
    let count = world.boss_bar_count();
    if count == 0 {
        run.succeed(
            translatable("commands.bossbar.list.bars.none", Vec::new()),
            false,
        );
    } else {
        run.succeed(
            translatable(
                "commands.bossbar.list.bars.some",
                vec![
                    NbtSummaryValue::Int(i32::try_from(count).unwrap_or(i32::MAX)),
                    format_list(world.boss_bar_states().map(event_display_name).collect()),
                ],
            ),
            false,
        );
    }
    i32::try_from(count).unwrap_or(i32::MAX)
}

fn set_name(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
) -> Result<(i32, CommandBossBarMutation), NbtSummaryValue> {
    let existing = event(parse, world)?;
    let event_id = existing.id.clone();
    let existing_name = existing.name.clone();
    let color = existing.color;
    let resolved = world.with_entity_reads(|reads| {
        super::text::resolve(component_argument(parse, "name"), source, at, reads, None)
    })?;
    let name =
        crate::commands::component_with_registry(resolved, world.plugins().content_registry());
    if existing_name == name {
        return Err(translatable(
            "commands.bossbar.set.name.unchanged",
            Vec::new(),
        ));
    }
    let mutation = CommandBossBarMutation::SetName {
        id: event_id.clone(),
        name: name.clone(),
    };
    run.succeed(
        translatable(
            "commands.bossbar.set.name.success",
            vec![display_name(&event_id, &name, color)],
        ),
        true,
    );
    Ok((0, mutation))
}

fn set_color(
    run: &mut CommandRun,
    parse: &ParseResult,
    world: &ServerWorld,
    color: BossBarColor,
) -> Result<(i32, CommandBossBarMutation), NbtSummaryValue> {
    let event = event(parse, world)?;
    if event.color == color {
        return Err(translatable(
            "commands.bossbar.set.color.unchanged",
            Vec::new(),
        ));
    }
    let mutation = CommandBossBarMutation::SetColor {
        id: event.id.clone(),
        color,
    };
    run.succeed(
        translatable(
            "commands.bossbar.set.color.success",
            vec![display_name(&event.id, &event.name, color)],
        ),
        true,
    );
    Ok((0, mutation))
}

fn set_overlay(
    run: &mut CommandRun,
    parse: &ParseResult,
    world: &ServerWorld,
    overlay: BossBarOverlay,
) -> Result<(i32, CommandBossBarMutation), NbtSummaryValue> {
    let event = event(parse, world)?;
    if event.overlay == overlay {
        return Err(translatable(
            "commands.bossbar.set.style.unchanged",
            Vec::new(),
        ));
    }
    let mutation = CommandBossBarMutation::SetOverlay {
        id: event.id.clone(),
        overlay,
    };
    run.succeed(
        translatable(
            "commands.bossbar.set.style.success",
            vec![event_display_name(event)],
        ),
        true,
    );
    Ok((0, mutation))
}

fn set_value(
    run: &mut CommandRun,
    parse: &ParseResult,
    world: &ServerWorld,
) -> Result<(i32, CommandBossBarMutation), NbtSummaryValue> {
    let event = event(parse, world)?;
    let value = integer_argument(parse, "value");
    if event.value == value {
        return Err(translatable(
            "commands.bossbar.set.value.unchanged",
            Vec::new(),
        ));
    }
    let mutation = CommandBossBarMutation::SetValue {
        id: event.id.clone(),
        value,
    };
    run.succeed(
        translatable(
            "commands.bossbar.set.value.success",
            vec![event_display_name(event), NbtSummaryValue::Int(value)],
        ),
        true,
    );
    Ok((value, mutation))
}

fn set_max(
    run: &mut CommandRun,
    parse: &ParseResult,
    world: &ServerWorld,
) -> Result<(i32, CommandBossBarMutation), NbtSummaryValue> {
    let event = event(parse, world)?;
    let max = integer_argument(parse, "max");
    if event.max == max {
        return Err(translatable(
            "commands.bossbar.set.max.unchanged",
            Vec::new(),
        ));
    }
    let mutation = CommandBossBarMutation::SetMax {
        id: event.id.clone(),
        max,
    };
    run.succeed(
        translatable(
            "commands.bossbar.set.max.success",
            vec![event_display_name(event), NbtSummaryValue::Int(max)],
        ),
        true,
    );
    Ok((max, mutation))
}

fn set_visible(
    run: &mut CommandRun,
    parse: &ParseResult,
    world: &ServerWorld,
) -> Result<(i32, CommandBossBarMutation), NbtSummaryValue> {
    let event = event(parse, world)?;
    let Some(ArgumentValue::Boolean(visible)) = parse.argument("visible") else {
        unreachable!("visible is a boolean argument");
    };
    if event.visible == *visible {
        return Err(translatable(
            if *visible {
                "commands.bossbar.set.visibility.unchanged.visible"
            } else {
                "commands.bossbar.set.visibility.unchanged.hidden"
            },
            Vec::new(),
        ));
    }
    let mutation = CommandBossBarMutation::SetVisible {
        id: event.id.clone(),
        visible: *visible,
    };
    run.succeed(
        translatable(
            if *visible {
                "commands.bossbar.set.visible.success.visible"
            } else {
                "commands.bossbar.set.visible.success.hidden"
            },
            vec![event_display_name(event)],
        ),
        true,
    );
    Ok((0, mutation))
}

fn set_players(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
    has_targets: bool,
) -> Result<(i32, CommandBossBarMutation), NbtSummaryValue> {
    let existing = event(parse, world)?;
    let event_id = existing.id.clone();
    let display = event_display_name(existing);
    let current = existing.players.iter().copied().collect::<FxHashSet<_>>();
    world.with_entity_reads(|reads| {
        let targets = if has_targets {
            let Some(ArgumentValue::Players(selector)) = parse.argument("targets") else {
                unreachable!("targets is a player argument");
            };
            select_player_states(selector, source, at, reads)?
        } else {
            Vec::new()
        };
        let players = targets.iter().map(|target| target.id).collect::<Vec<_>>();
        let replacement = players.iter().copied().collect::<FxHashSet<_>>();
        if current == replacement {
            return Err(translatable(
                "commands.bossbar.set.players.unchanged",
                Vec::new(),
            ));
        }
        let mutation = CommandBossBarMutation::SetPlayers {
            id: event_id,
            players,
        };
        if targets.is_empty() {
            run.succeed(
                translatable("commands.bossbar.set.players.success.none", vec![display]),
                true,
            );
        } else {
            run.succeed(
                translatable(
                    "commands.bossbar.set.players.success.some",
                    vec![
                        display,
                        NbtSummaryValue::Int(i32::try_from(targets.len()).unwrap_or(i32::MAX)),
                        player_list(
                            targets
                                .iter()
                                .map(|target| target.user())
                                .collect::<Vec<_>>()
                                .iter(),
                            reads.scoreboard,
                        ),
                    ],
                ),
                true,
            );
        }
        Ok((i32::try_from(targets.len()).unwrap_or(i32::MAX), mutation))
    })
}

fn get_value(run: &mut CommandRun, parse: &ParseResult, world: &ServerWorld) -> Outcome {
    let event = event(parse, world)?;
    run.succeed(
        translatable(
            "commands.bossbar.get.value",
            vec![event_display_name(event), NbtSummaryValue::Int(event.value)],
        ),
        true,
    );
    Ok(event.value)
}

fn get_max(run: &mut CommandRun, parse: &ParseResult, world: &ServerWorld) -> Outcome {
    let event = event(parse, world)?;
    run.succeed(
        translatable(
            "commands.bossbar.get.max",
            vec![event_display_name(event), NbtSummaryValue::Int(event.max)],
        ),
        true,
    );
    Ok(event.max)
}

fn get_visible(run: &mut CommandRun, parse: &ParseResult, world: &ServerWorld) -> Outcome {
    let event = event(parse, world)?;
    run.succeed(
        translatable(
            if event.visible {
                "commands.bossbar.get.visible.visible"
            } else {
                "commands.bossbar.get.visible.hidden"
            },
            vec![event_display_name(event)],
        ),
        true,
    );
    Ok(i32::from(event.visible))
}

fn get_players(run: &mut CommandRun, parse: &ParseResult, world: &ServerWorld) -> Outcome {
    let id = identifier_argument(parse, "id");
    let (event, online_players) = world.boss_bar_online_players(id).ok_or_else(|| {
        translatable(
            "commands.bossbar.unknown",
            vec![NbtSummaryValue::string(id)],
        )
    })?;
    if online_players.is_empty() {
        run.succeed(
            translatable(
                "commands.bossbar.get.players.none",
                vec![event_display_name(event)],
            ),
            true,
        );
    } else {
        run.succeed(
            translatable(
                "commands.bossbar.get.players.some",
                vec![
                    event_display_name(event),
                    NbtSummaryValue::Int(i32::try_from(online_players.len()).unwrap_or(i32::MAX)),
                    player_list(online_players.iter(), world.scoreboard()),
                ],
            ),
            true,
        );
    }
    Ok(i32::try_from(online_players.len()).unwrap_or(i32::MAX))
}

/// `wrapStores`'s bossbar arm (`ExecuteCommand.java:296-303`), whose
/// `BossBarCommands.getBossBar` runs while the modifier does.
pub(in crate::commands) fn resolve_store_boss_bar(
    id: &str,
    into_value: bool,
    world: &ServerWorld,
) -> Result<StoreTarget, NbtSummaryValue> {
    if !world.has_boss_bar(id) {
        return Err(translatable(
            "commands.bossbar.unknown",
            vec![NbtSummaryValue::string(id)],
        ));
    }
    Ok(StoreTarget::BossBar {
        id: id.to_owned(),
        into_value,
    })
}

fn event<'a>(
    parse: &ParseResult,
    world: &'a ServerWorld,
) -> Result<&'a CommandBossBarState, NbtSummaryValue> {
    let id = identifier_argument(parse, "id");
    world.boss_bar_state(id).ok_or_else(|| {
        translatable(
            "commands.bossbar.unknown",
            vec![NbtSummaryValue::string(id)],
        )
    })
}

fn player_list<'a>(
    players: impl Iterator<Item = &'a crate::admin::NameAndId>,
    board: &Scoreboard,
) -> NbtSummaryValue {
    format_list(
        players
            .map(|player| {
                team_player_display_name_nbt(
                    &CommandPlayerView {
                        name: player.name.clone(),
                        uuid: player.id,
                    },
                    board,
                )
            })
            .collect(),
    )
}

fn event_display_name(event: &CommandBossBarState) -> NbtSummaryValue {
    display_name(&event.id, &event.name, event.color)
}

fn display_name(id: &str, name: &RichTextComponent, color: BossBarColor) -> NbtSummaryValue {
    // CustomBossEvent.java:84-91 and BossEvent.java:90-109.
    let mut style = ComponentStyle::with_chat_formatting_color(color_formatting(color))
        .expect("boss bar colors are chat colors");
    style.hover_event = Some(show_text(id));
    style.insertion = Some(id.to_string().into());
    component_translatable_nbt(
        "chat.square_brackets",
        vec![name.component().clone()],
        &style,
    )
    .expect("boss bar display-name style is representable")
}

fn color_formatting(color: BossBarColor) -> ChatFormatting {
    match color {
        BossBarColor::Pink => ChatFormatting::Red,
        BossBarColor::Blue => ChatFormatting::Blue,
        BossBarColor::Red => ChatFormatting::DarkRed,
        BossBarColor::Green => ChatFormatting::Green,
        BossBarColor::Yellow => ChatFormatting::Yellow,
        BossBarColor::Purple => ChatFormatting::DarkBlue,
        BossBarColor::White => ChatFormatting::White,
    }
}

fn color_for_name(name: &str) -> BossBarColor {
    bbb_facts::boss_bar_styles::VANILLA_BOSS_BAR_COLORS
        .iter()
        .find(|entry| entry.serialized_name == name)
        .map(|entry| entry.color)
        .expect("the generated bossbar subtree names a catalog color")
}

fn overlay_for_name(name: &str) -> BossBarOverlay {
    bbb_facts::boss_bar_styles::VANILLA_BOSS_BAR_OVERLAYS
        .iter()
        .find(|entry| entry.serialized_name == name)
        .map(|entry| entry.overlay)
        .expect("the generated bossbar subtree names a catalog overlay")
}
