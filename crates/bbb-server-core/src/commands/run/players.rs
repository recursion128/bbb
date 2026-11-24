//! The player-management family: list, op/deop, kick, ban/pardon, banlist, whitelist.

use bbb_protocol::{component_literal_nbt, NbtSummaryValue};

use crate::commands::parse::{ArgumentValue, ParseResult};
use crate::commands::{
    component, team_player_display_name_nbt, translatable, CommandCx, CommandOrigin,
    CommandPlayerView, CommandSource,
};
use crate::world::ServerWorld;

use super::{
    ban_reason_message, player_name_matches, players_argument, profiles_argument, text_argument,
    view_of, CommandRun, Outcome,
};

/// `ListPlayersCommand.format` (ListPlayersCommand.java:31-37).
pub(super) fn list_players(run: &mut CommandRun, world: &ServerWorld, with_uuids: bool) -> i32 {
    let entries = world
        .command_players()
        .map(|player| {
            let view = view_of(player);
            if with_uuids {
                // `commands.list.nameAndId` (:25-29) takes the undecorated name
                // and the id spelled out.
                translatable(
                    "commands.list.nameAndId",
                    vec![
                        component_literal_nbt(player.name),
                        component_literal_nbt(player.id.to_string()),
                    ],
                )
            } else {
                team_player_display_name_nbt(&view, world.scoreboard())
            }
        })
        .collect::<Vec<_>>();
    let count = i32::try_from(world.player_count()).unwrap_or(i32::MAX);
    run.succeed(
        translatable(
            "commands.list.players",
            vec![
                NbtSummaryValue::Int(count),
                NbtSummaryValue::Int(i32::try_from(world.max_players()).unwrap_or(i32::MAX)),
                super::format_list(entries),
            ],
        ),
        false,
    );
    count
}

/// `OpCommand.opPlayers` (OpCommand.java:37-54) and `DeOpCommands.deopPlayers`
/// (DeOpCommands.java:30-48): one message per player the list actually changed
/// for, and a refusal when it changed for nobody.
pub(super) fn operate(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
    grant: bool,
) -> Outcome {
    let targets = profiles_argument(parse, "targets", source, cx.world, &cx.origin)?;
    let mut count = 0;
    for target in &targets {
        if cx.world.admin_lists().operators.contains_key(&target.id) == grant {
            continue;
        }
        if grant {
            cx.world.op_player(target.clone(), cx.events);
            run.succeed(
                translatable(
                    "commands.op.success",
                    vec![component_literal_nbt(&target.name)],
                ),
                true,
            );
        } else {
            cx.world.deop_player(target.id, cx.events);
            run.succeed(
                translatable(
                    "commands.deop.success",
                    // `deopPlayers` names `players.iterator().next()` rather than
                    // the player it just removed (DeOpCommands.java:38), so every
                    // line of a multi-target deop names the first target.
                    vec![component_literal_nbt(
                        &targets
                            .first()
                            .expect("the loop is over a non-empty list")
                            .name,
                    )],
                ),
                true,
            );
        }
        count += 1;
    }
    if count == 0 {
        return Err(translatable(
            if grant {
                "commands.op.failed"
            } else {
                "commands.deop.failed"
            },
            Vec::new(),
        ));
    }
    if !grant {
        // `deopPlayers` ends by dropping anyone the change locked out (:46).
        cx.world.kick_unlisted_players(cx.events);
    }
    Ok(count)
}

/// `KickCommand.kickPlayers` (KickCommand.java:35-55). The singleplayer-owner
/// refusals cannot be reached: this server publishes no world and has no
/// singleplayer owner, so every target is kickable.
pub(super) fn kick(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
) -> Outcome {
    let (targets, reason) = {
        let (world, at) = (&mut *cx.world, &cx.origin);
        let targets = players_argument(parse, "targets", source, world, at)?;
        let reason = match parse.argument("reason") {
            Some(ArgumentValue::Message(_)) => world.with_entity_reads(|reads| {
                super::text::message(parse, "reason", source, at, reads)
            })?,
            // `Component.translatable("multiplayer.disconnect.kicked")` (:26).
            _ => translatable("multiplayer.disconnect.kicked", Vec::new()),
        };
        Ok::<_, NbtSummaryValue>((targets, reason))
    }?;
    for target in &targets {
        let view = CommandPlayerView {
            name: target.name.clone(),
            uuid: target.id,
        };
        let kick_reason = crate::commands::component_with_registry(
            reason.clone(),
            cx.world.plugins().content_registry(),
        );
        cx.world
            .disconnect_command_player(target, kick_reason, cx.events);
        run.succeed(
            translatable(
                "commands.kick.success",
                vec![
                    team_player_display_name_nbt(&view, cx.world.scoreboard()),
                    reason.clone(),
                ],
            ),
            true,
        );
    }
    Ok(i32::try_from(targets.len()).unwrap_or(i32::MAX))
}

// BanPlayerCommands.java:30-43; BanIpCommands.java:31-58.
fn ban_reason(
    parse: &ParseResult,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
) -> Result<Option<bbb_protocol::JavaString>, NbtSummaryValue> {
    if parse.argument("reason").is_none() {
        return Ok(None);
    }
    let component = world
        .with_entity_reads(|reads| super::text::message(parse, "reason", source, at, reads))?;
    Ok(Some(bbb_protocol::JavaString::from(
        bbb_server_datapack::language::component_plain_text(&component),
    )))
}

/// `BanPlayerCommands.banPlayers` (BanPlayerCommands.java:36-58).
pub(super) fn ban_players(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
) -> Outcome {
    let targets = profiles_argument(parse, "targets", source, cx.world, &cx.origin)?;
    let reason = ban_reason(parse, source, cx.world, &cx.origin)?;
    let mut count = 0;
    for target in targets {
        if cx.world.admin_lists().player_bans.contains_key(&target.id) {
            continue;
        }
        cx.world
            .ban_player(target.clone(), source.name.clone(), reason.clone());
        run.succeed(
            translatable(
                "commands.ban.success",
                vec![
                    component_literal_nbt(&target.name),
                    ban_reason_message(reason.as_ref()),
                ],
            ),
            true,
        );
        if let Some(player) = cx
            .world
            .command_player_by_id(target.id)
            .map(|player| player.target())
        {
            cx.world.disconnect_command_player(
                &player,
                component(translatable("multiplayer.disconnect.banned", Vec::new())),
                cx.events,
            );
        }
        count += 1;
    }
    if count == 0 {
        return Err(translatable("commands.ban.failed", Vec::new()));
    }
    Ok(count)
}

/// `BanIpCommands.banIpOrName` (BanIpCommands.java:38-70): a literal address is
/// banned directly, a name is resolved to the address that player is connected
/// from, and anything else is refused.
pub(super) fn ban_ip(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
) -> Outcome {
    let target = text_argument(parse, "target");
    let (address, affected) = {
        let (world, at) = (&mut *cx.world, &cx.origin);
        let reason = ban_reason(parse, source, world, at)?;
        let address = if target
            .with_utf8(crate::admin::valid_ip_literal)
            .unwrap_or(false)
        {
            target.to_string()
        } else {
            // `banIpOrName` (BanIpCommands.java:42) resolves the word through the
            // same case-insensitive player lookup.
            match world
                .command_players()
                .find(|player| player_name_matches(player.name, target))
                .and_then(|player| player.address.map(str::to_owned))
            {
                Some(address) => address,
                None => return Err(translatable("commands.banip.invalid", Vec::new())),
            }
        };
        if world.admin_lists().ip_bans.contains_key(&address) {
            return Err(translatable("commands.banip.failed", Vec::new()));
        }
        let affected = world
            .command_players()
            .filter(|player| player.address == Some(address.as_str()))
            .map(|player| {
                (
                    player.target(),
                    team_player_display_name_nbt(&view_of(player), world.scoreboard()),
                )
            })
            .collect::<Vec<_>>();
        Ok(((address, reason), affected))
    }?;
    let (address, reason) = address;
    cx.world
        .ban_ip(address.clone(), source.name.clone(), reason.clone());
    run.succeed(
        translatable(
            "commands.banip.success",
            vec![
                component_literal_nbt(&address),
                ban_reason_message(reason.as_ref()),
            ],
        ),
        true,
    );
    if !affected.is_empty() {
        run.succeed(
            translatable(
                "commands.banip.info",
                vec![
                    NbtSummaryValue::Int(i32::try_from(affected.len()).unwrap_or(i32::MAX)),
                    // `EntitySelector.joinNames`
                    // (commands/arguments/selector/EntitySelector.java:287-289) is
                    // `ComponentUtils.formatList` over the display names.
                    super::format_list(affected.iter().map(|(_, name)| name.clone()).collect()),
                ],
            ),
            true,
        );
    }
    for (player, _) in &affected {
        cx.world.disconnect_command_player(
            player,
            component(translatable("multiplayer.disconnect.ip_banned", Vec::new())),
            cx.events,
        );
    }
    Ok(i32::try_from(affected.len()).unwrap_or(i32::MAX))
}

/// `PardonCommand.pardonPlayers` (PardonCommand.java:30-47).
pub(super) fn pardon_players(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
) -> Outcome {
    let targets = profiles_argument(parse, "targets", source, cx.world, &cx.origin)?;
    let mut count = 0;
    for target in targets {
        if !cx.world.admin_lists().player_bans.contains_key(&target.id) {
            continue;
        }
        cx.world.pardon_player(target.id);
        run.succeed(
            translatable(
                "commands.pardon.success",
                vec![component_literal_nbt(&target.name)],
            ),
            true,
        );
        count += 1;
    }
    if count == 0 {
        return Err(translatable("commands.pardon.failed", Vec::new()));
    }
    Ok(count)
}

/// `PardonIpCommand.unban` (PardonIpCommand.java:30-43).
pub(super) fn pardon_ip(
    run: &mut CommandRun,
    parse: &ParseResult,
    cx: &mut CommandCx<'_>,
) -> Outcome {
    let address = text_argument(parse, "target");
    if !address
        .with_utf8(crate::admin::valid_ip_literal)
        .unwrap_or(false)
    {
        return Err(translatable("commands.pardonip.invalid", Vec::new()));
    }
    let address_text = address
        .with_utf8(str::to_owned)
        .expect("a valid IP address is ASCII");
    if !cx.world.admin_lists().ip_bans.contains_key(&address_text) {
        return Err(translatable("commands.pardonip.failed", Vec::new()));
    }
    cx.world.pardon_ip(&address_text);
    run.succeed(
        translatable(
            "commands.pardonip.success",
            vec![component_literal_nbt(address)],
        ),
        true,
    );
    Ok(1)
}

/// `BanListCommands.showList` (BanListCommands.java:27-41): a header and then
/// one message per entry, or the empty-list message. The display name is the
/// banned player's name (UserBanListEntry.java:34-37) or the banned address
/// itself (IpBanListEntry.java:20-22), and the combined form is the player list
/// followed by the address list (:20).
pub(super) fn ban_list(
    run: &mut CommandRun,
    world: &ServerWorld,
    players: bool,
    addresses: bool,
) -> i32 {
    let mut entries = Vec::new();
    if players {
        for entry in world.admin_lists().player_bans.values() {
            entries.push((
                component_literal_nbt(&entry.user.name),
                entry.ban.source.clone(),
                entry.ban.reason.clone(),
            ));
        }
    }
    if addresses {
        for (address, ban) in &world.admin_lists().ip_bans {
            entries.push((
                component_literal_nbt(address),
                ban.source.clone(),
                ban.reason.clone(),
            ));
        }
    }
    if entries.is_empty() {
        run.succeed(translatable("commands.banlist.none", Vec::new()), false);
        return 0;
    }
    let count = i32::try_from(entries.len()).unwrap_or(i32::MAX);
    run.succeed(
        translatable("commands.banlist.list", vec![NbtSummaryValue::Int(count)]),
        false,
    );
    for (display_name, source, reason) in entries {
        run.succeed(
            translatable(
                "commands.banlist.entry",
                vec![
                    display_name,
                    component_literal_nbt(source),
                    ban_reason_message(reason.as_ref()),
                ],
            ),
            false,
        );
    }
    count
}

/// `WhitelistCommand.enableWhitelist` and `disableWhitelist`
/// (WhitelistCommand.java:119-138).
pub(super) fn set_whitelist_enabled(
    run: &mut CommandRun,
    cx: &mut CommandCx<'_>,
    enabled: bool,
) -> Outcome {
    if cx.world.admin_settings().use_allowlist == enabled {
        return Err(translatable(
            if enabled {
                "commands.whitelist.alreadyOn"
            } else {
                "commands.whitelist.alreadyOff"
            },
            Vec::new(),
        ));
    }
    cx.world.set_allowlist_enabled(enabled);
    run.succeed(
        translatable(
            if enabled {
                "commands.whitelist.enabled"
            } else {
                "commands.whitelist.disabled"
            },
            Vec::new(),
        ),
        true,
    );
    if enabled {
        cx.world.kick_unlisted_players(cx.events);
    }
    Ok(1)
}

/// `WhitelistCommand.showList` (:140-149): the names are joined by a plain
/// `", "` rather than by a component list.
pub(super) fn whitelist_list(run: &mut CommandRun, world: &ServerWorld) -> i32 {
    let allowlist = &world.admin_lists().allowlist;
    if allowlist.is_empty() {
        run.succeed(translatable("commands.whitelist.none", Vec::new()), false);
        return 0;
    }
    let mut names = Vec::new();
    for (index, entry) in allowlist.values().enumerate() {
        if index != 0 {
            names.extend([u16::from(b','), u16::from(b' ')]);
        }
        names.extend_from_slice(entry.name.units());
    }
    let count = i32::try_from(allowlist.len()).unwrap_or(i32::MAX);
    run.succeed(
        translatable(
            "commands.whitelist.list",
            vec![NbtSummaryValue::Int(count), component_literal_nbt(names)],
        ),
        false,
    );
    count
}

/// `WhitelistCommand.reload` (:71-76).
///
/// `PlayerList.reloadWhiteList` (PlayerList.java:641-642) has an empty body in
/// 26.1, so the only thing this command does is report and then drop whoever
/// the list no longer allows.
pub(super) fn reload_whitelist(run: &mut CommandRun, cx: &mut CommandCx<'_>) -> i32 {
    run.succeed(
        translatable("commands.whitelist.reloaded", Vec::new()),
        true,
    );
    cx.world.kick_unlisted_players(cx.events);
    1
}

/// `WhitelistCommand.addPlayers` (:78-96).
pub(super) fn whitelist_add(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
) -> Outcome {
    let targets = profiles_argument(parse, "targets", source, cx.world, &cx.origin)?;
    let mut count = 0;
    for target in targets {
        if cx.world.admin_lists().allowlist.contains_key(&target.id) {
            continue;
        }
        cx.world.allowlist_add(target.clone());
        run.succeed(
            translatable(
                "commands.whitelist.add.success",
                vec![component_literal_nbt(&target.name)],
            ),
            true,
        );
        count += 1;
    }
    if count == 0 {
        return Err(translatable("commands.whitelist.add.failed", Vec::new()));
    }
    Ok(count)
}

/// `WhitelistCommand.removePlayers` (:98-117).
pub(super) fn whitelist_remove(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
) -> Outcome {
    let targets = profiles_argument(parse, "targets", source, cx.world, &cx.origin)?;
    let mut count = 0;
    for target in targets {
        if !cx.world.admin_lists().allowlist.contains_key(&target.id) {
            continue;
        }
        cx.world.allowlist_remove(target.id);
        run.succeed(
            translatable(
                "commands.whitelist.remove.success",
                vec![component_literal_nbt(&target.name)],
            ),
            true,
        );
        count += 1;
    }
    if count == 0 {
        return Err(translatable("commands.whitelist.remove.failed", Vec::new()));
    }
    cx.world.kick_unlisted_players(cx.events);
    Ok(count)
}
