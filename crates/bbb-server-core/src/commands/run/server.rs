//! The server-state family: datapack list, reload, seed, defaultgamemode,
//! difficulty, gamerule, save-all/on/off, setworldspawn, stop, version and
//! transfer.

use bbb_protocol::{
    chat_formattings::ChatFormatting,
    component_appended_nbt, component_literal_nbt, component_translatable_nbt,
    difficulties::VANILLA_DIFFICULTIES,
    game_rules::{vanilla_game_rule, GameRuleValue},
    game_types::GameType,
    packets::BlockPos,
    ComponentClickEvent, ComponentHoverEvent, ComponentStyle, NbtSummaryValue, StyledTextRun,
};
use bbb_server_datapack::pack::ServerPackSource;
use bbb_server_datapack::pack_create::CreatePackRefusal;
use bbb_server_datapack::pack_repository::{position_insert, selection_config};

use bbb_server_types::save::Java26LevelKey;

use crate::admin::AdminEffect;
use crate::commands::parse::{ArgumentValue, ParseResult};
use crate::commands::types::{CommandDatapack, CommandDebugProfilerSample, CommandPackDescription};
use crate::commands::{
    chat_formatting_style, format_list, translatable, CommandCx, CommandOrigin, CommandSource,
};
use crate::world::WorldEvent;
use crate::world::{CommandDatapacks, ServerWorld};

use super::{
    component_argument, difficulty_display_name, difficulty_entry, game_mode_argument,
    integer_argument, player_targets, text_argument, CommandRun, Outcome,
};

/// `DataPackCommand.listPacks` (DataPackCommand.java:231-233): both faces,
/// return the sum of the two counts.
pub(super) fn list_packs(run: &mut CommandRun, world: &ServerWorld) -> i32 {
    list_enabled_packs(run, world) + list_available_packs(run, world)
}

/// `DataPackCommand.listAvailablePacks` (DataPackCommand.java:235-256): the
/// unselected packs whose requested features the world has (`:241-243`).
pub(super) fn list_available_packs(run: &mut CommandRun, world: &ServerWorld) -> i32 {
    let packs = world.command_datapacks();
    let unselected = packs
        .available
        .iter()
        .filter(|pack| !packs.is_selected(&pack.id) && packs.features_available(pack))
        .cloned()
        .collect::<Vec<_>>();
    list_pack_side(
        run,
        &unselected,
        false,
        "commands.datapack.list.available.none",
        "commands.datapack.list.available.success",
    )
}

/// `DataPackCommand.listEnabledPacks` (DataPackCommand.java:258-274).
pub(super) fn list_enabled_packs(run: &mut CommandRun, world: &ServerWorld) -> i32 {
    list_pack_side(
        run,
        &world.command_datapacks().selected,
        true,
        "commands.datapack.list.enabled.none",
        "commands.datapack.list.enabled.success",
    )
}

fn list_pack_side(
    run: &mut CommandRun,
    packs: &[CommandDatapack],
    enabled: bool,
    none_key: &str,
    success_key: &str,
) -> i32 {
    if packs.is_empty() {
        run.succeed(translatable(none_key, Vec::new()), false);
        return 0;
    }
    let count = i32::try_from(packs.len()).unwrap_or(i32::MAX);
    let links = packs
        .iter()
        .map(|pack| pack_chat_link(pack, enabled))
        .collect();
    run.succeed(
        translatable(
            success_key,
            vec![NbtSummaryValue::Int(count), format_list(links)],
        ),
        false,
    );
    count
}

/// `Pack.getChatLink` (`Pack.java:84-86`) through
/// `PackLocationInfo.createChatLink` (`PackLocationInfo.java:13-20`).
fn pack_chat_link(pack: &CommandDatapack, enabled: bool) -> NbtSummaryValue {
    let decorated = component_translatable_nbt(
        "pack.nameAndSource",
        vec![
            component_literal_nbt(&pack.id),
            translatable(pack_source_key(pack.source), Vec::new()),
        ],
        &chat_formatting_style(ChatFormatting::Gray),
    )
    .expect("pack.nameAndSource is representable");
    let hover = component_appended_nbt(
        &ComponentStyle::default(),
        vec![
            pack_title(pack),
            component_literal_nbt("\n"),
            pack_description(pack),
        ],
    )
    .expect("the chat-link hover is representable");
    let color = if enabled {
        ChatFormatting::Green
    } else {
        ChatFormatting::Red
    };
    component_translatable_nbt(
        "chat.square_brackets",
        vec![decorated],
        &ComponentStyle {
            hover_event: Some(ComponentHoverEvent::ShowText {
                value: Vec::new(),
                component: Some(hover),
            }),
            insertion: Some(escape_if_required(&pack.id).into()),
            ..chat_formatting_style(color)
        },
    )
    .expect("the chat-link brackets are representable")
}

fn pack_source_key(source: ServerPackSource) -> &'static str {
    match source {
        ServerPackSource::BuiltIn => "pack.source.builtin",
        ServerPackSource::Feature => "pack.source.feature",
        ServerPackSource::World => "pack.source.world",
    }
}

fn pack_title(pack: &CommandDatapack) -> NbtSummaryValue {
    match pack.description {
        CommandPackDescription::Vanilla => translatable("dataPack.vanilla.name", Vec::new()),
        CommandPackDescription::Literal(_) => component_literal_nbt(&pack.id),
    }
}

fn pack_description(pack: &CommandDatapack) -> NbtSummaryValue {
    match &pack.description {
        CommandPackDescription::Vanilla => translatable("dataPack.vanilla.description", Vec::new()),
        CommandPackDescription::Literal(text) => component_literal_nbt(text),
    }
}

/// `StringArgumentType.escapeIfRequired` (`PackLocationInfo.java:17`).
pub(in crate::commands) fn escape_if_required(id: &str) -> String {
    if id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '+'))
    {
        return id.to_string();
    }
    let mut escaped = String::from("\"");
    for c in id.chars() {
        match c {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            c => escaped.push(c),
        }
    }
    escaped.push('"');
    escaped
}

/// Where the enable arm puts the pack in the selected list; one per
/// `DataPackCommand.Inserter` the register method builds
/// (`DataPackCommand.java:102-133`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PackInsertion {
    /// `p.getDefaultPosition().insert(l, p, Pack::selectionConfig, false)` (`:107`).
    Default,
    /// `l.add(l.indexOf(existing) + 1, p)` (`:116`).
    After,
    /// `l.add(l.indexOf(existing), p)` (`:127`).
    Before,
    /// `List::add` (`:131`).
    Last,
    /// `l.add(0, p)` (`:132`).
    First,
}

/// `DataPackCommand.enablePack` (DataPackCommand.java:213-220): the new order
/// is computed from the live selection, announced, and then handed to
/// `ReloadCommand.reloadPacks`; the result is the size of the new selection.
pub(super) fn enable_pack(
    run: &mut CommandRun,
    parse: &ParseResult,
    cx: &mut CommandCx<'_>,
    insertion: PackInsertion,
) -> Outcome {
    let selected = {
        let packs = cx.world.command_datapacks();
        let unopened = get_pack(&packs, text_argument(parse, "name"), true)?;
        let mut selected = packs.selected.clone();
        let index = match insertion {
            PackInsertion::Default => {
                let (_, position) = selection_config(unopened.source);
                position_insert(&mut selected, unopened.clone(), position, |pack| {
                    selection_config(pack.source)
                });
                None
            }
            PackInsertion::First => Some(0),
            PackInsertion::Last => Some(selected.len()),
            // `getPack(c, "existing", false)` runs inside the inserter, so a bad
            // `existing` refuses after `name` was accepted and before the success
            // line goes out. It must name a selected pack, so `indexOf` finds it.
            PackInsertion::After | PackInsertion::Before => {
                let existing = get_pack(&packs, text_argument(parse, "existing"), false)?;
                let at = selected
                    .iter()
                    .position(|pack| pack.id == existing.id)
                    .expect("an already-enabled pack is in the selected list");
                Some(at + usize::from(insertion == PackInsertion::After))
            }
        };
        if let Some(index) = index {
            selected.insert(index, unopened.clone());
        }
        run.succeed(
            translatable(
                "commands.datapack.modify.enable",
                vec![pack_chat_link(unopened, true)],
            ),
            true,
        );
        Ok::<_, NbtSummaryValue>(selected)
    }?;
    Ok(reload_selection(cx, selected))
}

/// `DataPackCommand.disablePack` (DataPackCommand.java:222-229). The chat link
/// is built with `enabled = true` here too (`:226`), so the pack that is going
/// away is still named in green.
pub(super) fn disable_pack(
    run: &mut CommandRun,
    parse: &ParseResult,
    cx: &mut CommandCx<'_>,
) -> Outcome {
    let selected = {
        let packs = cx.world.command_datapacks();
        let unopened = get_pack(&packs, text_argument(parse, "name"), false)?;
        let mut selected = packs.selected.clone();
        selected.retain(|pack| pack.id != unopened.id);
        run.succeed(
            translatable(
                "commands.datapack.modify.disable",
                vec![pack_chat_link(unopened, true)],
            ),
            true,
        );
        Ok::<_, NbtSummaryValue>(selected)
    }?;
    Ok(reload_selection(cx, selected))
}

/// `DataPackCommand.createPack` (DataPackCommand.java:165-211): the two
/// `FileUtil` name tests, the directory that is already there, the metadata
/// encode and the write, then the success line (`:209-210`); each refusal
/// (`:64-78`) carries the id as the primitive `with` entry
/// `translatableEscape` keeps.
pub(super) fn create_pack(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
) -> Outcome {
    let name = text_argument(parse, "id");
    let description = {
        let (world, at) = (&mut *cx.world, &cx.origin);
        world.with_entity_reads(|reads| {
            super::text::resolve(
                component_argument(parse, "description"),
                source,
                at,
                reads,
                None,
            )
        })
    }?;
    let id = component_literal_nbt(name);
    name.with_utf8(|name| {
        bbb_server_datapack::pack_create::create_pack(cx.world.datapacks_dir(), name, &description)
    })
    .unwrap_or(Err(CreatePackRefusal::InvalidName))
    .map_err(|refusal| match refusal {
        CreatePackRefusal::InvalidName => {
            translatable("commands.datapack.create.invalid_name", vec![id.clone()])
        }
        CreatePackRefusal::InvalidFullName => translatable(
            "commands.datapack.create.invalid_full_name",
            vec![id.clone()],
        ),
        CreatePackRefusal::AlreadyExists => {
            translatable("commands.datapack.create.already_exists", vec![id.clone()])
        }
        CreatePackRefusal::MetadataEncodeFailure(message) => translatable(
            "commands.datapack.create.metadata_encode_failure",
            vec![id.clone(), component_literal_nbt(&message)],
        ),
        CreatePackRefusal::IoFailure => {
            translatable("commands.datapack.create.io_failure", vec![id.clone()])
        }
    })?;
    run.succeed(
        translatable("commands.datapack.create.success", vec![id]),
        true,
    );
    Ok(1)
}

fn reload_selection(cx: &mut CommandCx<'_>, selected: Vec<CommandDatapack>) -> i32 {
    cx.events.push(WorldEvent::ReloadRequested {
        selected: Some(selected.iter().map(|pack| pack.id.clone()).collect()),
    });
    i32::try_from(selected.len()).unwrap_or(i32::MAX)
}

/// `DataPackCommand.getPack` (DataPackCommand.java:276-300): the refusal ladder
/// both arms share, in its order.
fn get_pack<'a>(
    packs: &'a CommandDatapacks,
    id: &bbb_protocol::JavaString,
    enabling: bool,
) -> Result<&'a CommandDatapack, NbtSummaryValue> {
    let literal = component_literal_nbt(id);
    let Some(pack) = id.with_utf8(|id| packs.pack(id)).flatten() else {
        return Err(translatable("commands.datapack.unknown", vec![literal]));
    };
    let enabled = packs.is_selected(&pack.id);
    if enabling && enabled {
        return Err(translatable(
            "commands.datapack.enable.failed",
            vec![literal],
        ));
    }
    if !enabling && !enabled {
        return Err(translatable(
            "commands.datapack.disable.failed",
            vec![literal],
        ));
    }
    if !enabling && !pack.requested_features.is_empty() && pack.source == ServerPackSource::Feature
    {
        return Err(translatable(
            "commands.datapack.disable.failed.feature",
            vec![literal],
        ));
    }
    if !packs.features_available(pack) {
        return Err(translatable(
            "commands.datapack.enable.failed.no_flags",
            vec![literal, component_literal_nbt(missing_flags(pack, packs))],
        ));
    }
    Ok(pack)
}

/// `FeatureFlags.printMissingFlags` (FeatureFlags.java:22-26). Vanilla joins a
/// `HashSet` of ids; these are sorted, which is the one order that is
/// derivable.
fn missing_flags(pack: &CommandDatapack, packs: &CommandDatapacks) -> String {
    pack.requested_features
        .iter()
        .filter(|feature| !packs.enabled_features.contains(feature))
        .cloned()
        .collect::<Vec<_>>()
        .join(", ")
}

/// `DebugCommand.start` (`DebugCommand.java:71-80`).
pub(super) fn start_debug_profiler(run: &mut CommandRun, cx: &mut CommandCx<'_>) -> Outcome {
    if cx.world.debug_profiler_view().running {
        return Err(translatable("commands.debug.alreadyRunning", Vec::new()));
    }
    cx.world.start_debug_time_profiler();
    run.succeed(translatable("commands.debug.started", Vec::new()), true);
    Ok(0)
}

/// `DebugCommand.stop` (`DebugCommand.java:82-98`).
pub(super) fn stop_debug_profiler(run: &mut CommandRun, cx: &mut CommandCx<'_>) -> Outcome {
    let profiler = cx.world.debug_profiler_view();
    if !profiler.running {
        return Err(translatable("commands.debug.notRunning", Vec::new()));
    }
    let (seconds, ticks, tps) = debug_profiler_results(profiler.sample);
    cx.world.stop_debug_time_profiler();
    run.succeed(
        translatable(
            "commands.debug.stopped",
            vec![
                component_literal_nbt(format_java_fixed_2(seconds)),
                NbtSummaryValue::Int(ticks),
                component_literal_nbt(format_java_fixed_2(tps)),
            ],
        ),
        true,
    );
    Ok(tps as i32)
}

/// `String.format(Locale.ROOT, "%.2f", value)` (`DebugCommand.java:93`).
fn format_java_fixed_2(value: f64) -> String {
    if value.is_nan() {
        "NaN".to_string()
    } else if value == f64::INFINITY {
        "Infinity".to_string()
    } else if value == f64::NEG_INFINITY {
        "-Infinity".to_string()
    } else if value.abs() * 100.0 >= i64::MAX as f64 {
        format!("{value:.2}")
    } else {
        let formatted = crate::tick_rate::java_fixed_decimals(value, 2);
        if value.is_sign_negative() && !formatted.starts_with('-') {
            format!("-{formatted}")
        } else {
            formatted
        }
    }
}

/// `TimeProfiler.stop` (`MinecraftServer.java:2243-2279`) through
/// `ProfileResults.getNanoDuration` / `getTickDuration`
/// (`ProfileResults.java:21-27`). A missing sample is
/// `EmptyProfileResults.EMPTY` (`MinecraftServer.java:2031-2033`).
fn debug_profiler_results(sample: Option<CommandDebugProfilerSample>) -> (f64, i32, f64) {
    const NANOS_PER_SECOND: f64 = 1_000_000_000.0;
    let Some(sample) = sample else {
        return (0.0, 0, f64::NAN);
    };
    let seconds = (sample.now_nanos - sample.start_nanos) as f64 / NANOS_PER_SECOND;
    let ticks = sample.now_tick - sample.start_tick;
    (seconds, ticks, ticks as f64 / seconds)
}

/// `ReloadCommand.register` (ReloadCommand.java:44-49): the answer goes out
/// before the reload runs, and the reload itself is the server's work.
pub(super) fn reload(run: &mut CommandRun, cx: &mut CommandCx<'_>) -> i32 {
    run.succeed(translatable("commands.reload.success", Vec::new()), true);
    cx.events
        .push(WorldEvent::ReloadRequested { selected: None });
    0
}

/// `SeedCommand.register` (SeedCommand.java:12-17): the seed is reported
/// through `ComponentUtils.copyOnClickText` (ComponentUtils.java:158-168) and
/// returned as an `int`.
pub(super) fn seed(run: &mut CommandRun, world: &ServerWorld) -> i32 {
    let text = world.seed().to_string();
    let copyable = super::styled_literal(
        &text,
        &ComponentStyle {
            click_event: Some(ComponentClickEvent::CopyToClipboard {
                value: text.clone().into(),
            }),
            hover_event: Some(ComponentHoverEvent::ShowText {
                value: vec![StyledTextRun {
                    text: "chat.copy.click".to_string(),
                    style: ComponentStyle::default(),
                }],
                component: Some(translatable("chat.copy.click", Vec::new())),
            }),
            insertion: Some(text.clone().into()),
            ..chat_formatting_style(ChatFormatting::Green)
        },
    );
    // `ComponentUtils.wrapInSquareBrackets` (:140-142).
    let brackets = translatable("chat.square_brackets", vec![copyable]);
    run.succeed(translatable("commands.seed.success", vec![brackets]), false);
    world.seed() as i32
}

/// `DefaultGameModeCommands.setMode` (DefaultGameModeCommands.java:20-26):
/// the write, the report, and the number of sessions
/// `enforceGameTypeForPlayers(getForcedGameType())` moved.
pub(super) fn set_default_game_mode(
    run: &mut CommandRun,
    parse: &ParseResult,
    cx: &mut CommandCx<'_>,
) -> i32 {
    let game_type = game_mode_argument(parse, "gamemode");
    let moved = cx.world.set_default_game_type(game_type, cx.events);
    run.succeed(
        translatable(
            "commands.defaultgamemode.success",
            vec![game_type_long_display_name(game_type)],
        ),
        true,
    );
    moved
}

/// `GameType.getLongDisplayName` (GameType.java:54-56), which the constructor
/// built as `Component.translatable("gameMode." + name)` (:38) over the same
/// serialized name `GameModeArgument` parsed (:36, :45-52).
pub(super) fn game_type_long_display_name(game_type: GameType) -> NbtSummaryValue {
    translatable(
        format!("gameMode.{}", game_type.serialized_name()),
        Vec::new(),
    )
}

/// `DifficultyCommand.register` (DifficultyCommand.java:25-29): the query
/// reports the display name and returns the difficulty id.
pub(super) fn query_difficulty(run: &mut CommandRun, world: &ServerWorld) -> i32 {
    run.succeed(
        translatable(
            "commands.difficulty.query",
            vec![difficulty_display_name(world.difficulty())],
        ),
        false,
    );
    difficulty_entry(world.difficulty()).id
}

/// `DifficultyCommand.setDifficulty` (:32-41): a difficulty already in force is
/// a failure carrying the serialized name, and a change reports the display
/// name and returns zero.
pub(super) fn set_difficulty(run: &mut CommandRun, name: &str, cx: &mut CommandCx<'_>) -> Outcome {
    let entry = VANILLA_DIFFICULTIES
        .iter()
        .find(|entry| entry.serialized_name == name)
        .expect("the derived tree names one literal per Difficulty value");
    if cx.world.difficulty() == entry.difficulty {
        return Err(translatable(
            "commands.difficulty.failure",
            vec![component_literal_nbt(entry.serialized_name)],
        ));
    }
    cx.world
        .set_difficulty_ignoring_lock(entry.difficulty, cx.events);
    run.succeed(
        translatable(
            "commands.difficulty.success",
            vec![difficulty_display_name(entry.difficulty)],
        ),
        true,
    );
    Ok(0)
}

/// `GameRuleCommand.queryRule` (GameRuleCommand.java:43-47).
pub(super) fn query_game_rule(run: &mut CommandRun, rule: &str, world: &ServerWorld) -> Outcome {
    let (id, resource_id) = game_rule_ids(rule);
    let value = world
        .game_rule(&resource_id)
        .expect("the derived tree only names registered game rules");
    run.succeed(
        translatable(
            "commands.gamerule.query",
            vec![
                component_literal_nbt(id),
                component_literal_nbt(value.serialize()),
            ],
        ),
        false,
    );
    Ok(game_rule_command_result(value))
}

/// `GameRuleCommand.setRule` (:35-41).
pub(super) fn set_game_rule(
    run: &mut CommandRun,
    parse: &ParseResult,
    rule: &str,
    cx: &mut CommandCx<'_>,
) -> Outcome {
    let (id, resource_id) = game_rule_ids(rule);
    let entry =
        vanilla_game_rule(&resource_id).expect("the derived tree only names registered game rules");
    let value = match parse.argument("value") {
        Some(ArgumentValue::Boolean(value)) => GameRuleValue::Boolean(*value),
        Some(ArgumentValue::Integer(value)) => GameRuleValue::Integer(*value),
        other => unreachable!("a game rule value is a boolean or an integer, not {other:?}"),
    };
    cx.world
        .set_game_rule_value(entry.resource_id, value, cx.events);
    run.succeed(
        translatable(
            "commands.gamerule.set",
            vec![
                component_literal_nbt(id),
                component_literal_nbt(value.serialize()),
            ],
        ),
        true,
    );
    Ok(game_rule_command_result(value))
}

/// `GameRules.registerBoolean` (GameRules.java:167-179) and `registerInteger`
/// (:189-203) name the two command-result functions.
fn game_rule_command_result(value: GameRuleValue) -> i32 {
    match value {
        GameRuleValue::Boolean(value) => i32::from(value),
        GameRuleValue::Integer(value) => value,
    }
}

/// The literal is either the rule's short id or its full identifier
/// (GameRuleCommand.java:20-22); the feedback always names the short one
/// (`GameRule.id`, GameRule.java:51-53).
fn game_rule_ids(rule: &str) -> (String, String) {
    match rule.split_once(':') {
        Some((_, path)) => (path.to_string(), rule.to_string()),
        None => (rule.to_string(), format!("minecraft:{rule}")),
    }
}

/// `SaveAllCommand.saveAll` (SaveAllCommand.java:23-33). The save itself is the
/// world's work, so — as with `/reload` — the command reports the accepted save
/// rather than a finished one, and `commands.save.failed` belongs to whoever
/// runs it.
pub(super) fn save_all(run: &mut CommandRun, cx: &mut CommandCx<'_>, flush: bool) -> i32 {
    run.succeed(translatable("commands.save.saving", Vec::new()), false);
    cx.world
        .apply_admin_effect(AdminEffect::Save { flush }, cx.events);
    run.succeed(translatable("commands.save.success", Vec::new()), true);
    1
}

/// `SaveOnCommand` (SaveOnCommand.java:13-22) and `SaveOffCommand`
/// (SaveOffCommand.java:13-22): `setAutoSave` answers whether it changed
/// anything, and no change is the refusal.
pub(super) fn set_auto_save(
    run: &mut CommandRun,
    cx: &mut CommandCx<'_>,
    enabled: bool,
) -> Outcome {
    if !cx.world.set_auto_save(enabled) {
        return Err(translatable(
            if enabled {
                "commands.save.alreadyOn"
            } else {
                "commands.save.alreadyOff"
            },
            Vec::new(),
        ));
    }
    run.succeed(
        translatable(
            if enabled {
                "commands.save.enabled"
            } else {
                "commands.save.disabled"
            },
            Vec::new(),
        ),
        true,
    );
    Ok(1)
}

/// `SetPlayerIdleTimeoutCommand.setIdleTimeout`
/// (SetPlayerIdleTimeoutCommand.java:21-30): no guard on the value it is given,
/// and the disabled wording is the zero the argument's own `integer(0)` floor
/// leaves reachable.
pub(super) fn set_idle_timeout(
    run: &mut CommandRun,
    parse: &ParseResult,
    cx: &mut CommandCx<'_>,
) -> Outcome {
    let minutes = integer_argument(parse, "minutes");
    cx.world.set_player_idle_timeout(minutes);
    if minutes > 0 {
        run.succeed(
            translatable(
                "commands.setidletimeout.success",
                vec![NbtSummaryValue::Int(minutes)],
            ),
            true,
        );
    } else {
        run.succeed(
            translatable("commands.setidletimeout.success.disabled", Vec::new()),
            true,
        );
    }
    Ok(minutes)
}

/// `StopCommand.register` (StopCommand.java:10-14).
pub(super) fn stop(run: &mut CommandRun, cx: &mut CommandCx<'_>) -> i32 {
    run.succeed(translatable("commands.stop.stopping", Vec::new()), true);
    cx.world.apply_admin_effect(AdminEffect::Stop, cx.events);
    1
}

/// `TransferCommand`'s default port (TransferCommand.java:28), the value the
/// node with no `<port>` passes.
const DEFAULT_TRANSFER_PORT: i32 = 25565;

/// `VersionCommand.register`'s executor (VersionCommand.java:19-24) over
/// `dumpVersion` (`:28-39`).
///
/// Every line goes out through `sendSystemMessage` rather than `sendSuccess`,
/// which is a different channel and not merely a different wording:
/// `CommandSourceStack.sendSystemMessage` (CommandSourceStack.java:453-462) does
/// not consult `acceptsSuccess`, so `/gamerule send_command_feedback false`
/// silences no part of this command.
///
/// The nine values are `SharedConstants.getCurrentVersion()`, which on a
/// released build is `version.json` (DetectedVersion.java:32-42) -- the same
/// document `bbb_protocol`'s version constants are checked against.
pub(super) fn report_version(run: &mut CommandRun) -> i32 {
    // `VersionCommand.java:13`, sent at `:21`.
    run.system_message(translatable("commands.version.header", Vec::new()));
    // `:30-31`: `WorldVersion.id()` and `name()`.
    run.system_message(translatable(
        "commands.version.id",
        vec![component_literal_nbt(bbb_protocol::MC_VERSION)],
    ));
    run.system_message(translatable(
        "commands.version.name",
        vec![component_literal_nbt(bbb_protocol::MC_VERSION)],
    ));
    // `:32-33`: `DataVersion.version()` and `series()`.
    run.system_message(translatable(
        "commands.version.data",
        vec![NbtSummaryValue::Int(bbb_protocol::MC_DATA_VERSION)],
    ));
    run.system_message(translatable(
        "commands.version.series",
        vec![component_literal_nbt(bbb_protocol::MC_DATA_VERSION_SERIES)],
    ));
    // `:34`: the protocol version twice, once as the number and once as the
    // `"0x" + Integer.toHexString(..)` string beside it.
    run.system_message(translatable(
        "commands.version.protocol",
        vec![
            NbtSummaryValue::Int(bbb_protocol::PROTOCOL_VERSION),
            component_literal_nbt(format!("0x{:x}", bbb_protocol::PROTOCOL_VERSION)),
        ],
    ));
    // `:35`: `Component.translationArg(Date)`, a literal of `Date.toString()`.
    run.system_message(translatable(
        "commands.version.build_time",
        vec![component_literal_nbt(
            bbb_facts::mc_build_time_java_date_string(),
        )],
    ));
    // `VersionCommand.java:36-37`, each through `PackFormat.toString()`
    // (PackFormat.java:97-101).
    run.system_message(translatable(
        "commands.version.pack.resource",
        vec![component_literal_nbt(
            bbb_protocol::MC_RESOURCE_PACK_FORMAT.to_vanilla_string(),
        )],
    ));
    run.system_message(translatable(
        "commands.version.pack.data",
        vec![component_literal_nbt(
            bbb_protocol::MC_DATA_PACK_FORMAT.to_vanilla_string(),
        )],
    ));
    // `:38`, over the two constants at `:14-15`.
    run.system_message(translatable(
        if bbb_protocol::MC_STABLE {
            "commands.version.stable.yes"
        } else {
            "commands.version.stable.no"
        },
        Vec::new(),
    ));
    1
}

/// `TransferCommand.transfer` (TransferCommand.java:55-71).
///
/// The empty-players refusal at `:56-58` is unreachable through the published
/// tree and is implemented anyway: the two shorter nodes pass
/// `List.of(getPlayerOrException())`, which is never empty, and the third goes
/// through `EntityArgument.getPlayers`, which has already thrown
/// `argument.entity.notfound.player` for an empty result
/// (EntityArgument.java:89-96).
pub(super) fn transfer(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
    ported: bool,
    targeted: bool,
) -> Outcome {
    let hostname = text_argument(parse, "hostname");
    // `25565` is the literal the shortest node passes (`:28`); the other two
    // read the `<port>` argument.
    let port = if ported {
        integer_argument(parse, "port")
    } else {
        DEFAULT_TRANSFER_PORT
    };
    let (players, first_name) = player_targets(
        parse,
        targeted.then_some("players"),
        source,
        cx.world,
        &cx.origin,
    )?;
    if players.is_empty() {
        return Err(translatable(
            "commands.transfer.error.no_players",
            Vec::new(),
        ));
    }
    for player in &players {
        cx.world.transfer_player(player, hostname, port, cx.events);
    }
    let count = i32::try_from(players.len()).unwrap_or(i32::MAX);
    // The hostname is a `String` argument and the port an `int`, which
    // `TranslatableContents` serializes as the primitives they are rather than
    // as components (`PRIMITIVE_ARG_CODEC`, TranslatableContents.java:31-32).
    if players.len() == 1 {
        run.succeed(
            translatable(
                "commands.transfer.success.single",
                vec![
                    first_name.expect("one target was named"),
                    NbtSummaryValue::String(hostname.clone()),
                    NbtSummaryValue::Int(port),
                ],
            ),
            true,
        );
    } else {
        run.succeed(
            translatable(
                "commands.transfer.success.multiple",
                vec![
                    NbtSummaryValue::Int(count),
                    NbtSummaryValue::String(hostname.clone()),
                    NbtSummaryValue::Int(port),
                ],
            ),
            true,
        );
    }
    Ok(count)
}

/// `SetSpawnCommand.setSpawn` (`SetSpawnCommand.java:71-97`).
pub(super) fn set_spawnpoint(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
    named_targets: bool,
    explicit_pos: bool,
    explicit_rotation: bool,
) -> Outcome {
    let (players, first_name, spawn) = {
        let (world, at) = (&mut *cx.world, &cx.origin);
        let (players, first_name) =
            player_targets(parse, named_targets.then_some("targets"), source, world, at)?;
        let spawn = spawn_arguments(parse, at, explicit_pos, explicit_rotation)?;
        Ok((players, first_name, spawn))
    }?;
    let SpawnArguments {
        dimension,
        pos,
        yaw,
        pitch,
    } = spawn;
    for player in &players {
        cx.world
            .set_player_respawn_by_command(player, &dimension, pos, yaw, pitch);
    }
    let dimension_name = dimension.resource_id().to_string();
    if players.len() == 1 {
        run.succeed(
            translatable(
                "commands.spawnpoint.success.single",
                vec![
                    NbtSummaryValue::Int(pos.x),
                    NbtSummaryValue::Int(pos.y),
                    NbtSummaryValue::Int(pos.z),
                    NbtSummaryValue::Float(yaw.to_bits()),
                    NbtSummaryValue::Float(pitch.to_bits()),
                    NbtSummaryValue::String(dimension_name.into()),
                    first_name.expect("one target was named"),
                ],
            ),
            true,
        );
    } else {
        run.succeed(
            translatable(
                "commands.spawnpoint.success.multiple",
                vec![
                    NbtSummaryValue::Int(pos.x),
                    NbtSummaryValue::Int(pos.y),
                    NbtSummaryValue::Int(pos.z),
                    NbtSummaryValue::Float(yaw.to_bits()),
                    NbtSummaryValue::Float(pitch.to_bits()),
                    NbtSummaryValue::String(dimension_name.into()),
                    NbtSummaryValue::Int(i32::try_from(players.len()).unwrap_or(i32::MAX)),
                ],
            ),
            true,
        );
    }
    Ok(i32::try_from(players.len()).unwrap_or(i32::MAX))
}

/// `SetWorldSpawnCommand.setSpawn` (SetWorldSpawnCommand.java:33-52).
///
/// Bare `/setworldspawn` uses `BlockPos.containing(source.getPosition())` and
/// `WorldCoordinates.ZERO_ROTATION` (`:21`) with no spawnable-bounds check.
/// `pos` and `pos rotation` use `BlockPosArgument.getSpawnablePos` (`:24`,
/// `:27`), which refuses `argument.pos.outofbounds` when
/// `Level.isInSpawnableBounds` is false (BlockPosArgument.java:52-58).
pub(super) fn set_world_spawn(
    run: &mut CommandRun,
    parse: &ParseResult,
    cx: &mut CommandCx<'_>,
    explicit_pos: bool,
    explicit_rotation: bool,
) -> Outcome {
    let SpawnArguments {
        dimension,
        pos,
        yaw,
        pitch,
    } = spawn_arguments(parse, &cx.origin, explicit_pos, explicit_rotation)?;
    cx.world
        .set_world_spawn(&dimension, pos, yaw, pitch, cx.events);
    run.succeed(
        translatable(
            "commands.setworldspawn.success",
            vec![
                NbtSummaryValue::Int(pos.x),
                NbtSummaryValue::Int(pos.y),
                NbtSummaryValue::Int(pos.z),
                NbtSummaryValue::Float(yaw.to_bits()),
                NbtSummaryValue::Float(pitch.to_bits()),
                NbtSummaryValue::string(dimension.resource_id()),
            ],
        ),
        true,
    );
    Ok(1)
}

struct SpawnArguments {
    dimension: Java26LevelKey,
    pos: BlockPos,
    yaw: f32,
    pitch: f32,
}

fn spawn_arguments(
    parse: &ParseResult,
    at: &CommandOrigin,
    explicit_pos: bool,
    explicit_rotation: bool,
) -> Result<SpawnArguments, NbtSummaryValue> {
    let pos = if explicit_pos {
        spawnable_block_pos(parse, at)?
    } else {
        BlockPos {
            x: at.position[0].floor() as i32,
            y: at.position[1].floor() as i32,
            z: at.position[2].floor() as i32,
        }
    };
    let [yaw, pitch] = if explicit_rotation {
        match parse.argument("rotation") {
            Some(ArgumentValue::Rotation(coordinates)) => coordinates.rotation(at.rotation),
            other => unreachable!("rotation is a rotation argument, not {other:?}"),
        }
    } else {
        [0.0, 0.0]
    };
    Ok(SpawnArguments {
        dimension: at.level.clone(),
        pos,
        yaw: crate::world::wrap_degrees(yaw),
        pitch: pitch.clamp(-90.0, 90.0),
    })
}

/// `BlockPosArgument.getSpawnablePos` (BlockPosArgument.java:52-58).
fn spawnable_block_pos(
    parse: &ParseResult,
    at: &CommandOrigin,
) -> Result<BlockPos, NbtSummaryValue> {
    let Some(ArgumentValue::BlockPos(coordinates)) = parse.argument("pos") else {
        unreachable!("a block-position argument parsed as coordinates");
    };
    let pos = coordinates.block_pos(at);
    // `Level.isInSpawnableBounds` (Level.java:175-177): height inside
    // `isOutsideSpawnableHeight` (`:189-191`) and a column inside
    // `isInWorldBoundsHorizontal` (`:179-181`). Vanilla spells the literals
    // rather than the named constants, so this repeats them the same way.
    let inside_height = pos.y >= -20_000_000 && pos.y < 20_000_000;
    let inside_column =
        pos.x >= -30_000_000 && pos.z >= -30_000_000 && pos.x < 30_000_000 && pos.z < 30_000_000;
    if !(inside_height && inside_column) {
        return Err(translatable("argument.pos.outofbounds", Vec::new()));
    }
    Ok(pos)
}
