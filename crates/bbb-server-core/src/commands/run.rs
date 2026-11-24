//! What each implemented command does once the parse reached its node.
//!
//! One arm per executable node of the derived tree, matched on the canonical
//! path so an alias reaches the command it redirects to. Every message is the
//! translation key its vanilla owner passes to `sendSuccess` or to the
//! exception it throws.
//!
//! Bound `ChatType` decorations are resolved here; chat and system delivery
//! remain distinct so the session can apply the recipient's chat visibility.
//!
//! The other half of `sendSuccess` is not resolved here but is carried: the
//! `chat.type.admin` copy `sendSuccess(.., true)` sends to the other operators
//! (`CommandSourceStack.broadcastToAdmins`, CommandSourceStack.java:479-493)
//! travels as [`CommandFeedback::inform_admins`], which every `succeed` below
//! states explicitly with the `broadcast` argument its vanilla owner passes.
//! Delivery belongs to whoever holds the player list: the dispatcher knows
//! neither which other sessions are operators nor the two game rules the copy
//! is gated on.
//!
//! One `run/<family>.rs` per command family holds the executors; this file
//! keeps the types, [`run`] and the argument helpers every family shares.

mod advancement;
mod blocks;
pub(crate) use blocks::block_predicate_matches;
mod bossbar;
mod chat;
mod clear;
mod control;
mod data;
mod dialog;
mod effect;
mod enchant;
mod entity;
mod experience;
mod fetch_profile;
pub(crate) use fetch_profile::avatar_profile_feedback;
mod fillbiome;
mod force_load;
mod function;
mod game_mode;
mod give;
mod help;
mod item;
mod locate;
mod loot;
mod particle;
mod place;
mod players;
mod random;
mod recipe;
mod schedule;
mod scoreboard;
mod selector;
mod server;
mod sound;
mod spreadplayers;
mod stopwatch;
pub(super) mod summon;
mod team;
mod teleport;
mod tellraw;
mod text;
mod tick;
mod time;
mod title;
mod waypoint;
mod weather;
mod world_border;

pub(crate) use data::accessor_data;
pub(super) use function::{
    instantiate_function, modify_sender_for_execution, resolve_functions, InstantiationError,
};
pub(super) use selector::{select_damage_entity_states, select_player_states};
pub(super) use server::escape_if_required;

use super::types::CommandDamageEntityRef;
use crate::world::{EntityReads, ServerWorld};
use std::sync::Arc;

use crate::random::GameRandom;
use bbb_protocol::{
    chat_formattings::ChatFormatting,
    component_appended_nbt, component_literal_nbt,
    content_registry::ContentRegistry,
    difficulties::{Difficulty, VANILLA_DIFFICULTIES},
    entity_anchors::EntityAnchor,
    game_types::GameType,
    objective_render_types::ObjectiveRenderType,
    packets::BlockPos,
    parse_command_java_uuid, ComponentStyle, NbtSummaryValue, RichTextComponent,
};

use crate::admin::NameAndId;

use super::execution::{ExecutionControl, ExecutionSource, StoreTarget};
use super::parse::{
    ArgumentValue, Coordinates, ExecuteCondition, ExecuteStoreArm, ParseResult, ScoreTest,
    Selector, SelectorForm, SelectorKind,
};
use super::selector_options::{SelectorOrder, SelectorPredicate};
use super::types::{CommandExperienceKind, CommandPlayer, CommandWeatherKind};
use super::{
    catalog, chat_formatting_style, format_list, styled_literal, team_player_display_name_nbt,
    translatable, CommandCx, CommandOrigin, CommandPermission, CommandPlayerTarget,
    CommandPlayerView, CommandSource, CommandSourceIdentity,
};

use crate::world::ChatKind;
use advancement::advancement;
use blocks::{clone_blocks, fill_blocks, set_block, FillMode, FillPredicate, SetBlockMode};
use bossbar::bossbar;
use chat::{message_players, team_message};
use clear::clear_items;
use control::{return_fail, return_value};
use data::{data_command, function_arguments};
use dialog::{clear_dialog, show_dialog};
use effect::{clear_effects, give_effect};
use enchant::enchant_items;
use entity::{
    add_attribute_modifier, damage_entity, dismount, get_attribute_modifier_value,
    get_attribute_value, kill, list_tags, mount, remove_attribute_modifier, reset_attribute_base,
    rotate, set_attribute_base, set_tag, spectate, swing, DamageSourceShape,
};
use experience::{add_experience, query_experience, set_experience};
use fetch_profile::fetch_profile_entity;
use fillbiome::fill_biome;
use force_load::{change_force_load, list_force_load, query_force_load, remove_all_force_load};
use function::call_function;
use game_mode::set_game_mode;
use give::give_items;
pub(crate) use give::runtime_item_stack_display_name;
use help::help;
use item::{
    block_to_block, block_to_entities, entity_to_block, entity_to_entities, modify_block_item,
    modify_entity_item, set_block_item, set_entity_item,
};
use locate::locate;
pub(crate) use locate::{LocateKind, LocateRequest, LocateSearchResult};
use loot::drop_loot;
use particle::send_particles;
use place::{place_command, place_feature};
pub(crate) use place::{PlaceCommandKind, PlaceCommandRequest};
use players::{
    ban_ip, ban_list, ban_players, kick, list_players, operate, pardon_ip, pardon_players,
    reload_whitelist, set_whitelist_enabled, whitelist_add, whitelist_list, whitelist_remove,
};
use random::{random_sample, reset_all_sequences, reset_sequence};
use recipe::recipe;
use schedule::{clear_schedule, schedule_function};
use scoreboard::{
    add_objective, add_score, clear_display_slot, enable_trigger, get_score, list_objectives,
    list_tracked_player_scores, list_tracked_players, perform_operation, remove_objective,
    reset_all_scores, reset_score, set_display_slot, set_objective_display_auto_update,
    set_objective_display_name, set_objective_number_format, set_objective_render_type, set_score,
    set_score_display, set_score_number_format, trigger, NumberFormatArm, TriggerArm, Wildcard,
};
use server::{
    create_pack, disable_pack, enable_pack, list_available_packs, list_enabled_packs, list_packs,
    query_difficulty, query_game_rule, reload, report_version, save_all, seed, set_auto_save,
    set_default_game_mode, set_difficulty, set_game_rule, set_idle_timeout, set_spawnpoint,
    set_world_spawn, start_debug_profiler, stop, stop_debug_profiler, transfer, PackInsertion,
};
use sound::{play_sound, stop_sound};
use spreadplayers::spread_players;
use stopwatch::{create_stopwatch, query_stopwatch, remove_stopwatch, restart_stopwatch};
use summon::summon_entity;
use team::{
    add_team, empty_team, join_team, leave_team, list_team_members, list_teams, remove_team,
    set_team_affix, set_team_collision, set_team_color, set_team_display_name, set_team_flag,
    set_team_visibility, team_collision_rule, team_visibility,
};
use teleport::{teleport_to_entity, teleport_to_pos, FacingShape, TeleportTargets};
use tellraw::tellraw;
use tick::{
    set_freeze, set_ticking_rate, sprint, step, stop_sprinting, stop_stepping, tick_query,
    tick_time_argument,
};
use title::{clear_title, reset_title, set_title_times, show_title};
use waypoint::{list_waypoints, modify_waypoint};
use world_border::{
    get_world_border_size, set_world_border_center, set_world_border_damage_amount,
    set_world_border_damage_buffer, set_world_border_size, set_world_border_warning_distance,
    set_world_border_warning_time,
};

pub(crate) use recipe::RecipeCommandAction;
pub(crate) use tick::sprint_report;

/// Ordered broadcasts retain their channel for recipient visibility filtering.
#[derive(Debug, Clone, PartialEq)]
pub enum CommandBroadcast {
    /// `PlayerList.broadcastSystemMessage`.
    System(RichTextComponent),
    /// `PlayerList.broadcastChatMessage`.
    Chat(crate::world::OutgoingChat),
    /// `ServerPlayer.sendChatMessage` to one target, which `/msg` sends after
    /// the sender's own copy (MsgCommand.java:40-42).
    Whisper {
        player: CommandPlayerTarget,
        message: crate::world::OutgoingChat,
    },
}

/// Everything one command run produced.
///
/// The messages keep the order the command sent them in, because a command that
/// reports per target and then fails — `/op` over two players that are already
/// operators, for instance — has already sent everything before the refusal.
#[derive(Debug, Clone, PartialEq)]
pub struct CommandRun {
    pub(crate) content_registry: Arc<ContentRegistry>,
    /// The value the command returned, or `None` when it produced none.
    ///
    /// A command produces none in two cases. Almost always it ended in a
    /// `CommandSyntaxException` instead of returning. The other is a
    /// `CustomCommandExecutor` node: `BuildContexts` runs one itself
    /// (BuildContexts.java:102-107) rather than through
    /// `ContextChain.runExecutable`, so no `Command.run` is ever called for it
    /// and the value a caller eventually sees comes from the frame it reached
    /// (`FunctionCommand.decorateOutputIfNeeded`, FunctionCommand.java:144-151).
    pub result: Option<i32>,
    /// Successful command contexts delivered by `ExecutionCommandSource.resultConsumer`
    /// (`Commands.java:301`, `BuildContexts.java:101-116`).
    pub successful_contexts: i32,
    pub feedback: Vec<CommandFeedback>,
    pub broadcast: Vec<CommandBroadcast>,
    pub(crate) trace_errors: Vec<(usize, NbtSummaryValue)>,
    /// What the world emitted while the command ran, for tests that read the
    /// packets.
    #[cfg(test)]
    pub(crate) events: Vec<crate::world::WorldEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FetchProfileTarget {
    Player {
        uuid: uuid::Uuid,
    },
    Stored {
        dimension: bbb_server_types::save::Java26LevelKey,
        entity_id: i32,
        uuid: uuid::Uuid,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum FetchProfileResult {
    Avatar {
        /// Writer-canonical `ResolvableProfile.CODEC` NBT. The resolver folds
        /// decode-only strings and empty property lists into `FULL_CODEC` form.
        profile: NbtSummaryValue,
        display_name: NbtSummaryValue,
    },
    NotAvatar {
        display_name: NbtSummaryValue,
    },
    Unavailable,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CloneBlocksRequest {
    pub(crate) from_level: bbb_server_types::save::Java26LevelKey,
    pub(crate) to_level: bbb_server_types::save::Java26LevelKey,
    pub(crate) area_min: BlockPos,
    pub(crate) area_max: BlockPos,
    pub(crate) offset: BlockPos,
    pub(crate) filter: Option<super::parse::CommandBlockPredicate>,
    pub(crate) masked: bool,
    pub(crate) move_source: bool,
    pub(crate) strict: bool,
}

/// One message the source receives and the channel that sent it.
#[derive(Debug, Clone, PartialEq)]
pub struct CommandFeedback {
    pub message: RichTextComponent,
    pub kind: CommandFeedbackKind,
    /// `sendSuccess`'s `broadcast` argument (CommandSourceStack.java:464-477):
    /// whether the line is also copied to the other operators through
    /// `broadcastToAdmins` (`:479-493`) as a `chat.type.admin` translatable
    /// over the source's display name, styled gray and italic.
    ///
    /// The flag is the command's own -- `KillCommand.java:28` passes `true`
    /// and `ListPlayersCommand.java:35` passes `false` -- so it is recorded
    /// per line rather than per channel. Only [`CommandFeedbackKind::Success`]
    /// can inform admins; failure, system and chat channels have no broadcast half.
    ///
    /// Whether the copy is delivered at all is not decided here: `sendSuccess`
    /// reads `source.shouldInformAdmins()` (`:466`), `broadcastToAdmins` reads
    /// `SEND_COMMAND_FEEDBACK` for the operators and `LOG_ADMIN_COMMANDS` for
    /// the console (`:482-492`), and all three are the world's to answer.
    pub inform_admins: bool,
    /// The bound message a [`CommandFeedbackKind::Chat`] line is when its
    /// source is a player; [`Self::message`] is its decorated form for any
    /// other source (CommandSourceStack.java:442-451).
    pub chat: Option<Box<crate::world::OutgoingChat>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandFeedbackKind {
    /// `CommandSourceStack.sendSuccess` (CommandSourceStack.java:464-477): the
    /// message reaches the source exactly as the command built it.
    Success,
    /// `CommandSourceStack.sendFailure` (:495-499). The red wrapper that call
    /// puts around the message is already on [`CommandFeedback::message`], so
    /// the kind says which channel sent it rather than what still has to be
    /// done to it.
    Failure,
    /// `CommandSourceStack.sendSystemMessage` (CommandSourceStack.java:453-462),
    /// which is a third channel rather than a wording of the first: it never
    /// consults `acceptsSuccess`, so `send_command_feedback` does not silence
    /// it. `/version` is the only command this server publishes that reports on
    /// it (VersionCommand.java:21-22).
    System,
    /// `CommandSourceStack.sendChatMessage` (CommandSourceStack.java:442-451).
    Chat,
}

/// `CommandSourceStack.sendFailure` (CommandSourceStack.java:495-499): a
/// refusal travels as `Component.empty().append(message).withStyle(RED)`.
///
/// The wrapper is applied where the refusal is produced rather than where it is
/// sent, so every caller of this module holds a component it can put on the
/// wire unchanged — and a refusal that reached the client without the red root
/// would be a divergence no caller could repair on its own.
pub(crate) fn failure_feedback(message: NbtSummaryValue) -> CommandFeedback {
    failure_feedback_with_registry(message, ContentRegistry::vanilla())
}

fn failure_feedback_with_registry(
    message: NbtSummaryValue,
    registry: &ContentRegistry,
) -> CommandFeedback {
    CommandFeedback {
        message: super::component_with_registry(
            component_appended_nbt(&chat_formatting_style(ChatFormatting::Red), vec![message])
                .expect("the red refusal root accepts one sibling"),
            registry,
        ),
        kind: CommandFeedbackKind::Failure,
        inform_admins: false,
        chat: None,
    }
}

/// `CommandSourceStack.sendSuccess(message, broadcast)`
/// (CommandSourceStack.java:464-477).
pub(super) fn success_feedback(message: RichTextComponent, inform_admins: bool) -> CommandFeedback {
    CommandFeedback {
        message,
        kind: CommandFeedbackKind::Success,
        inform_admins,
        chat: None,
    }
}

impl Default for CommandRun {
    fn default() -> Self {
        Self::with_registry(ContentRegistry::vanilla())
    }
}

impl CommandRun {
    fn with_registry(registry: &Arc<ContentRegistry>) -> Self {
        Self {
            content_registry: Arc::clone(registry),
            result: None,
            successful_contexts: 0,
            feedback: Vec::new(),
            broadcast: Vec::new(),
            trace_errors: Vec::new(),
            #[cfg(test)]
            events: Vec::new(),
        }
    }

    /// The chat lines the world sent connection `connection` while the
    /// command ran (the fixture numbers its players from 1).
    #[cfg(test)]
    pub(crate) fn messages_to(&self, connection: u64) -> Vec<&RichTextComponent> {
        self.events
            .iter()
            .filter_map(|event| match event {
                crate::world::WorldEvent::Send {
                    connection_id,
                    packet: crate::connection::ClientboundPlayPacket::SystemChat(chat),
                } if connection_id.0 == connection => Some(&*chat.content),
                _ => None,
            })
            .collect()
    }

    /// The connections the world closed while the command ran, with the
    /// administrative reason each was given.
    #[cfg(test)]
    pub(crate) fn closed(&self) -> Vec<(u64, &RichTextComponent)> {
        self.events
            .iter()
            .filter_map(|event| match event {
                crate::world::WorldEvent::Close {
                    connection_id,
                    reason: crate::world::WorldCloseReason::Administrative(reason),
                } => Some((connection_id.0, reason)),
                _ => None,
            })
            .collect()
    }

    /// Every packet delivered to a connection, with its fixture connection id.
    #[cfg(test)]
    pub(crate) fn sends(&self) -> Vec<(u64, &crate::connection::ClientboundPlayPacket)> {
        self.events
            .iter()
            .flat_map(crate::world::WorldEvent::deliveries)
            .map(|(connection_id, packet)| (connection_id.0, packet))
            .collect()
    }

    /// Every play packet the world sent while the command ran.
    #[cfg(test)]
    pub(crate) fn packets(&self) -> Vec<&crate::connection::ClientboundPlayPacket> {
        self.events
            .iter()
            .filter_map(|event| match event {
                crate::world::WorldEvent::Broadcast { packet, .. }
                | crate::world::WorldEvent::Send { packet, .. } => Some(packet),
                _ => None,
            })
            .collect()
    }

    /// `CommandSourceStack.sendSuccess(message, broadcast)`
    /// (CommandSourceStack.java:464-477). `inform_admins` is the `broadcast`
    /// argument exactly as the vanilla owner passes it, which is why every
    /// caller spells it out rather than defaulting.
    fn succeed(&mut self, message: NbtSummaryValue, inform_admins: bool) {
        self.feedback.push(success_feedback(
            super::component_with_registry(message, &self.content_registry),
            inform_admins,
        ));
    }

    pub(super) fn fail(&mut self, message: NbtSummaryValue) {
        self.trace_errors
            .push((self.feedback.len(), message.clone()));
        self.feedback.push(failure_feedback_with_registry(
            message,
            &self.content_registry,
        ));
    }

    /// Folds what a later queue entry produced into what the entries before it
    /// did, which is how one typed command answers with one run however many
    /// entries the chain took.
    pub(crate) fn absorb(&mut self, other: Self) {
        if other.result.is_some() {
            self.result = other.result;
        }
        self.successful_contexts += other.successful_contexts;
        let offset = self.feedback.len();
        self.trace_errors.extend(
            other
                .trace_errors
                .into_iter()
                .map(|(index, message)| (index + offset, message)),
        );
        self.feedback.extend(other.feedback);
        self.broadcast.extend(other.broadcast);
    }

    /// `CommandSourceStack.sendSystemMessage` (CommandSourceStack.java:453-462).
    fn system_message(&mut self, message: NbtSummaryValue) {
        self.feedback.push(CommandFeedback {
            message: super::component_with_registry(message, &self.content_registry),
            kind: CommandFeedbackKind::System,
            inform_admins: false,
            chat: None,
        });
    }
}

/// What an executor answers: the command's return value, or the message of the
/// `CommandSyntaxException` it threw instead.
type Outcome = Result<i32, NbtSummaryValue>;

/// Runs the command the parse reached against the live world.
///
/// A `forked` chain swallows the command's exception rather than reporting it
/// (`ContextChain.runExecutable`, ContextChain.java:61-76).
pub(super) fn run(
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
    forked: bool,
) -> CommandRun {
    let mut run = CommandRun::with_registry(cx.world.plugins().content_registry());
    let outcome = dispatch(&mut run, parse, source, cx);
    match outcome {
        Ok(result) => {
            run.result = Some(result);
            run.successful_contexts = 1;
        }
        Err(message) if forked => {
            run.trace_errors.push((run.feedback.len(), message));
        }
        Err(message) => run.fail(message),
    }
    run
}

fn dispatch(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
) -> Outcome {
    let node = parse
        .executable_node()
        .expect("only a parse that reached a command is run");
    let path = catalog::canonical_path(node);
    match path.as_slice() {
        ["schedule", "function", "function", "time"]
        | ["schedule", "function", "function", "time", "replace"] => {
            schedule_function(run, parse, cx, true)
        }
        ["schedule", "function", "function", "time", "append"] => {
            schedule_function(run, parse, cx, false)
        }
        ["schedule", "clear", "function"] => clear_schedule(run, parse, cx),
        ["stopwatch", "create", "id"] => create_stopwatch(run, parse, cx),
        ["stopwatch", "restart", "id"] => restart_stopwatch(run, parse, cx),
        ["stopwatch", "remove", "id"] => remove_stopwatch(run, parse, cx),
        // `WeatherCommand.register` (`WeatherCommand.java:15-42`): three
        // literals, each with a bare arm and a `<duration>` one.
        ["weather", kind] => weather::set_weather(run, parse, cx, weather_kind(kind), false),
        ["weather", kind, "duration"] => {
            weather::set_weather(run, parse, cx, weather_kind(kind), true)
        }
        ["tick", "rate", "rate"] => Ok(set_ticking_rate(run, parse, cx)),
        ["tick", "freeze"] => Ok(set_freeze(run, cx, true)),
        ["tick", "unfreeze"] => Ok(set_freeze(run, cx, false)),
        ["tick", "step"] => Ok(step(run, cx, 1)),
        ["tick", "step", "time"] => Ok(step(run, cx, tick_time_argument(parse))),
        ["tick", "step", "stop"] => Ok(stop_stepping(run, cx)),
        ["tick", "sprint", "time"] => Ok(sprint(run, cx, tick_time_argument(parse))),
        ["tick", "sprint", "stop"] => Ok(stop_sprinting(run, cx)),
        ["debug", "start"] => start_debug_profiler(run, cx),
        ["debug", "stop"] => stop_debug_profiler(run, cx),
        ["defaultgamemode", "gamemode"] => Ok(set_default_game_mode(run, parse, cx)),
        ["difficulty", name] => set_difficulty(run, name, cx),
        ["gamerule", rule, "value"] => set_game_rule(run, parse, rule, cx),
        ["save-on"] => set_auto_save(run, cx, true),
        ["save-off"] => set_auto_save(run, cx, false),
        ["setidletimeout", "minutes"] => set_idle_timeout(run, parse, cx),
        // The three `/setworldspawn` executors (SetWorldSpawnCommand.java:18-29):
        // bare uses `BlockPos.containing(source.getPosition())` and
        // `WorldCoordinates.ZERO_ROTATION`; `pos` and `pos rotation` use
        // `BlockPosArgument.getSpawnablePos`.
        ["setworldspawn"] => set_world_spawn(run, parse, cx, false, false),
        ["setworldspawn", "pos"] => set_world_spawn(run, parse, cx, true, false),
        ["setworldspawn", "pos", "rotation"] => set_world_spawn(run, parse, cx, true, true),
        ["spawnpoint"] => set_spawnpoint(run, parse, source, cx, false, false, false),
        ["spawnpoint", "targets"] => set_spawnpoint(run, parse, source, cx, true, false, false),
        ["spawnpoint", "targets", "pos"] => {
            set_spawnpoint(run, parse, source, cx, true, true, false)
        }
        ["spawnpoint", "targets", "pos", "rotation"] => {
            set_spawnpoint(run, parse, source, cx, true, true, true)
        }
        // The three `/transfer` executors (TransferCommand.java:28, `:31-38`,
        // `:42-47`), which differ in whether a port and a player list were
        // typed.
        ["transfer", "hostname"] => transfer(run, parse, source, cx, false, false),
        ["transfer", "hostname", "port"] => transfer(run, parse, source, cx, true, false),
        ["transfer", "hostname", "port", "players"] => transfer(run, parse, source, cx, true, true),
        // `TimeCommand.addClockNodes` (`TimeCommand.java:64-112`) hangs the same
        // arms off the bare `/time` root and off `of <clock>`; the path decides
        // only which clock the arm runs on (`:48`, `:57`).
        ["time", rest @ ..] if !matches!(time_arm(rest), ["query", ..]) => {
            let explicit = matches!(rest, ["of", ..]);
            match time_arm(rest) {
                ["set", "time"] => time::set_clock_ticks(run, parse, cx, explicit),
                ["set", "timemarker"] => time::set_clock_time_marker(run, parse, cx, explicit),
                ["add", "time"] => time::add_clock_ticks(run, parse, cx, explicit),
                ["pause"] => time::set_clock_paused(run, parse, cx, explicit, true),
                ["resume"] => time::set_clock_paused(run, parse, cx, explicit, false),
                ["rate", "rate"] => time::set_clock_rate(run, parse, cx, explicit),
                other => unreachable!("/time has no {other:?} arm"),
            }
        }
        ["random", "value", "range"] | ["random", "value", "range", "sequence"] => {
            random_sample(run, parse, source, cx, false)
        }
        ["random", "roll", "range"] | ["random", "roll", "range", "sequence"] => {
            random_sample(run, parse, source, cx, true)
        }
        ["random", "reset", "*"]
        | ["random", "reset", "*", "seed"]
        | ["random", "reset", "*", "seed", "includeWorldSeed"]
        | ["random", "reset", "*", "seed", "includeWorldSeed", "includeSequenceId"] => {
            reset_all_sequences(run, parse, cx)
        }
        ["random", "reset", "sequence"]
        | ["random", "reset", "sequence", "seed"]
        | ["random", "reset", "sequence", "seed", "includeWorldSeed"]
        | ["random", "reset", "sequence", "seed", "includeWorldSeed", "includeSequenceId"] => {
            reset_sequence(run, parse, cx)
        }
        ["worldborder", "add", "distance"] => set_world_border_size(run, parse, cx, true, false),
        ["worldborder", "add", "distance", "time"] => {
            set_world_border_size(run, parse, cx, true, true)
        }
        ["worldborder", "set", "distance"] => set_world_border_size(run, parse, cx, false, false),
        ["worldborder", "set", "distance", "time"] => {
            set_world_border_size(run, parse, cx, false, true)
        }
        ["worldborder", "center", "pos"] => set_world_border_center(run, parse, cx),
        ["worldborder", "damage", "amount", "damagePerBlock"] => {
            set_world_border_damage_amount(run, parse, cx)
        }
        ["worldborder", "damage", "buffer", "distance"] => {
            set_world_border_damage_buffer(run, parse, cx)
        }
        ["worldborder", "warning", "distance", "distance"] => {
            set_world_border_warning_distance(run, parse, cx)
        }
        ["worldborder", "warning", "time", "time"] => set_world_border_warning_time(run, parse, cx),
        ["op", "targets"] => operate(run, parse, source, cx, true),
        ["deop", "targets"] => operate(run, parse, source, cx, false),
        ["kick", "targets"] | ["kick", "targets", "reason"] => kick(run, parse, source, cx),
        ["ban", "targets"] | ["ban", "targets", "reason"] => ban_players(run, parse, source, cx),
        ["ban-ip", "target"] | ["ban-ip", "target", "reason"] => ban_ip(run, parse, source, cx),
        ["pardon", "targets"] => pardon_players(run, parse, source, cx),
        ["pardon-ip", "target"] => pardon_ip(run, parse, cx),
        ["whitelist", "on"] => set_whitelist_enabled(run, cx, true),
        ["whitelist", "off"] => set_whitelist_enabled(run, cx, false),
        ["whitelist", "add", "targets"] => whitelist_add(run, parse, source, cx),
        ["whitelist", "remove", "targets"] => whitelist_remove(run, parse, source, cx),
        ["teammsg", "message"] => team_message(parse, source, cx),
        ["tellraw", "targets", "message"] => tellraw(parse, source, cx),
        // The two `/gamemode` executors (GameModeCommand.java:27 and `:30`),
        // which differ only in where the players come from.
        ["gamemode", "gamemode"] => set_game_mode(run, parse, source, cx, false),
        ["gamemode", "gamemode", "target"] => set_game_mode(run, parse, source, cx, true),
        // The two `/kill` executors (KillCommand.java:17-18), which differ only
        // in where the victims come from.
        ["kill"] => kill(run, parse, source, cx, false),
        ["kill", "targets"] => kill(run, parse, source, cx, true),
        // The four `/swing` executors (SwingCommand.java:25, `:28-30`). The
        // first two carry `MAIN_HAND` implicitly and the literals name a hand.
        ["swing"] => swing(run, parse, source, cx, false, false),
        ["swing", "targets"] | ["swing", "targets", "mainhand"] => {
            swing(run, parse, source, cx, true, false)
        }
        ["swing", "targets", "offhand"] => swing(run, parse, source, cx, true, true),
        // The four `/rotate` executors (RotateCommand.java:24, `:32-38`,
        // `:41-49`, `:55-59`): one `<rotation>` arm and three `facing ..` arms,
        // which are the same three shapes `/teleport ... facing ..` has.
        ["rotate", "target", "rotation"] => rotate(run, parse, source, cx, None),
        ["rotate", "target", "facing", "entity", "facingEntity"] => {
            rotate(run, parse, source, cx, Some(FacingShape::Entity))
        }
        ["rotate", "target", "facing", "entity", "facingEntity", "facingAnchor"] => {
            rotate(run, parse, source, cx, Some(FacingShape::AnchoredEntity))
        }
        ["rotate", "target", "facing", "facingLocation"] => {
            rotate(run, parse, source, cx, Some(FacingShape::Position))
        }
        // The three `/spectate` executors (SpectateCommand.java:28, `:31`,
        // `:34`), which differ in whether a target was named and in which
        // player is doing the spectating.
        ["spectate"] => spectate(run, parse, source, cx, false, false),
        ["spectate", "target"] => spectate(run, parse, source, cx, true, false),
        ["spectate", "target", "player"] => spectate(run, parse, source, cx, true, true),
        // The three `/tag` executors (TagCommand.java:32, `:40`, `:43`).
        ["tag", "targets", "add", "name"] => set_tag(run, parse, source, cx, true),
        ["tag", "targets", "remove", "name"] => set_tag(run, parse, source, cx, false),
        ["enchant", "targets", "enchantment"] | ["enchant", "targets", "enchantment", "level"] => {
            enchant_items(run, parse, source, cx)
        }
        waypoint_path @ ["waypoint", "modify", ..] => {
            modify_waypoint(run, waypoint_path, parse, source, cx)
        }
        bossbar_path @ ["bossbar", ..] => bossbar(run, parse, source, cx, bossbar_path),
        ["fetchprofile", "entity", "entity"] => fetch_profile_entity(run, parse, source, cx),
        data_path @ ["data", ..] => data_command(run, data_path, parse, source, cx),
        ["datapack", "create", "id", "description"] => create_pack(run, parse, source, cx),
        ["locate", kind @ ("biome" | "structure" | "poi"), _] => locate(run, parse, cx, kind),
        // The seven `/forceload` executors (ForceLoadCommand.java:42-49,
        // `:57-64`, `:67`, `:71` and `:74`): `add` and `remove` are one body
        // under a flag, and each takes the second corner or reuses the first.
        ["forceload", "add", "from"] | ["forceload", "add", "from", "to"] => {
            change_force_load(run, parse, cx, true)
        }
        ["forceload", "remove", "from"] | ["forceload", "remove", "from", "to"] => {
            change_force_load(run, parse, cx, false)
        }
        ["forceload", "remove", "all"] => remove_all_force_load(run, cx),
        ["fillbiome", "from", "to", "biome"] => fill_biome(run, parse, cx, false),
        ["fillbiome", "from", "to", "biome", "replace", "filter"] => {
            fill_biome(run, parse, cx, true)
        }
        // The five `/setblock` executors (SetBlockCommand.java:32-41, `:44-52`,
        // `:57-65`, `:70-78` and `:83-92`), which differ in the mode, the
        // filter and the `strict` flag alone.
        ["clone", ..] => clone_blocks(run, parse, cx, &path),
        ["setblock", "pos", "block"] | ["setblock", "pos", "block", "replace"] => {
            set_block(run, parse, cx, SetBlockMode::Replace, false)
        }
        ["setblock", "pos", "block", "destroy"] => {
            set_block(run, parse, cx, SetBlockMode::Destroy, false)
        }
        ["setblock", "pos", "block", "keep"] => {
            set_block(run, parse, cx, SetBlockMode::Keep, false)
        }
        ["setblock", "pos", "block", "strict"] => {
            set_block(run, parse, cx, SetBlockMode::Replace, true)
        }
        // The twelve `/fill` executors. `wrapWithMode` (FillCommand.java:96-141)
        // builds five of them twice -- once directly under `<block>` and once
        // under `replace <filter>` -- and `register` adds `replace` and `keep`
        // beside them (`:55-89`).
        ["fill", "from", "to", "block"] | ["fill", "from", "to", "block", "replace"] => {
            fill_blocks(
                run,
                parse,
                cx,
                FillMode::Replace,
                FillPredicate::None,
                false,
            )
        }
        ["fill", "from", "to", "block", "outline"] => fill_blocks(
            run,
            parse,
            cx,
            FillMode::Outline,
            FillPredicate::None,
            false,
        ),
        ["fill", "from", "to", "block", "hollow"] => {
            fill_blocks(run, parse, cx, FillMode::Hollow, FillPredicate::None, false)
        }
        ["fill", "from", "to", "block", "destroy"] => fill_blocks(
            run,
            parse,
            cx,
            FillMode::Destroy,
            FillPredicate::None,
            false,
        ),
        ["fill", "from", "to", "block", "strict"] => {
            fill_blocks(run, parse, cx, FillMode::Replace, FillPredicate::None, true)
        }
        ["fill", "from", "to", "block", "keep"] => fill_blocks(
            run,
            parse,
            cx,
            FillMode::Replace,
            FillPredicate::Empty,
            false,
        ),
        ["fill", "from", "to", "block", "replace", "filter"] => fill_blocks(
            run,
            parse,
            cx,
            FillMode::Replace,
            FillPredicate::Argument,
            false,
        ),
        ["fill", "from", "to", "block", "replace", "filter", "outline"] => fill_blocks(
            run,
            parse,
            cx,
            FillMode::Outline,
            FillPredicate::Argument,
            false,
        ),
        ["fill", "from", "to", "block", "replace", "filter", "hollow"] => fill_blocks(
            run,
            parse,
            cx,
            FillMode::Hollow,
            FillPredicate::Argument,
            false,
        ),
        ["fill", "from", "to", "block", "replace", "filter", "destroy"] => fill_blocks(
            run,
            parse,
            cx,
            FillMode::Destroy,
            FillPredicate::Argument,
            false,
        ),
        ["fill", "from", "to", "block", "replace", "filter", "strict"] => fill_blocks(
            run,
            parse,
            cx,
            FillMode::Replace,
            FillPredicate::Argument,
            true,
        ),
        ["place", "structure", ..] => place_command(run, parse, cx, "structure", false),
        ["place", "jigsaw", ..] => place_command(run, parse, cx, "jigsaw", false),
        ["place", "template", ..] => {
            place_command(run, parse, cx, "template", path.last() == Some(&"strict"))
        }
        ["place", "feature", "feature"] => place_feature(run, parse, cx, false),
        ["place", "feature", "feature", "pos"] => place_feature(run, parse, cx, true),
        ["recipe", "give", "targets", "recipe"] => {
            recipe(run, parse, source, cx, RecipeCommandAction::Give, false)
        }
        ["recipe", "give", "targets", "*"] => {
            recipe(run, parse, source, cx, RecipeCommandAction::Give, true)
        }
        ["recipe", "take", "targets", "recipe"] => {
            recipe(run, parse, source, cx, RecipeCommandAction::Take, false)
        }
        ["recipe", "take", "targets", "*"] => {
            recipe(run, parse, source, cx, RecipeCommandAction::Take, true)
        }
        advancement_path @ ["advancement", ..] => {
            advancement(run, parse, source, cx, advancement_path)
        }
        ["summon", "entity"] => summon_entity(run, parse, cx, false),
        ["summon", "entity", "pos"] | ["summon", "entity", "pos", "nbt"] => {
            summon_entity(run, parse, cx, true)
        }
        // The ten `/experience` executors (ExperienceCommand.java:35-42,
        // `:45-52`, `:56-63`, `:74-81`, `:84-91`, `:95-102`, `:113`, `:117`):
        // `add` and `set` each carry `Type.POINTS` twice -- once on the bare
        // `<amount>` and once under the `points` literal -- and `Type.LEVELS`
        // under `levels`; `query` names the type by its literal alone.
        ["experience", "add", "target", "amount"]
        | ["experience", "add", "target", "amount", "points"] => {
            add_experience(run, parse, source, cx, CommandExperienceKind::Points)
        }
        ["experience", "add", "target", "amount", "levels"] => {
            add_experience(run, parse, source, cx, CommandExperienceKind::Levels)
        }
        ["experience", "set", "target", "amount"]
        | ["experience", "set", "target", "amount", "points"] => {
            set_experience(run, parse, source, cx, CommandExperienceKind::Points)
        }
        ["experience", "set", "target", "amount", "levels"] => {
            set_experience(run, parse, source, cx, CommandExperienceKind::Levels)
        }
        ["attribute", "target", "attribute", "base", "set", "value"] => {
            set_attribute_base(run, parse, source, cx)
        }
        ["attribute", "target", "attribute", "base", "reset"] => {
            reset_attribute_base(run, parse, source, cx)
        }
        ["attribute", "target", "attribute", "modifier", "add", "id", "value", "add_value"] => {
            add_attribute_modifier(
                run,
                parse,
                source,
                cx,
                crate::entity::AttributeOperation::AddValue,
            )
        }
        ["attribute", "target", "attribute", "modifier", "add", "id", "value", "add_multiplied_base"] => {
            add_attribute_modifier(
                run,
                parse,
                source,
                cx,
                crate::entity::AttributeOperation::AddMultipliedBase,
            )
        }
        ["attribute", "target", "attribute", "modifier", "add", "id", "value", "add_multiplied_total"] => {
            add_attribute_modifier(
                run,
                parse,
                source,
                cx,
                crate::entity::AttributeOperation::AddMultipliedTotal,
            )
        }
        ["attribute", "target", "attribute", "modifier", "remove", "id"] => {
            remove_attribute_modifier(run, parse, source, cx)
        }
        // The eight `/teleport` executors (TeleportCommand.java:43-52, :56-60,
        // :66-75, :78-87, :95-104, :107-118, :124-133 and :139). Every one of
        // them reaches `teleportToPos` (`:174-215`) or `teleportToEntity`
        // (`:146-172`); what the node decides is where the targets come from
        // and which of the two optional shapes -- a `<rotation>` or a
        // `facing ..` -- came with them.
        ["teleport", "location"] => teleport_to_pos(
            run,
            parse,
            source,
            cx,
            TeleportTargets::SourceEntity,
            false,
            None,
        ),
        ["teleport", "destination"] => {
            teleport_to_entity(run, parse, source, cx, TeleportTargets::SourceEntity)
        }
        ["teleport", "targets", "destination"] => {
            teleport_to_entity(run, parse, source, cx, TeleportTargets::Argument)
        }
        ["teleport", "targets", "location"] => teleport_to_pos(
            run,
            parse,
            source,
            cx,
            TeleportTargets::Argument,
            false,
            None,
        ),
        ["teleport", "targets", "location", "rotation"] => teleport_to_pos(
            run,
            parse,
            source,
            cx,
            TeleportTargets::Argument,
            true,
            None,
        ),
        ["teleport", "targets", "location", "facing", "entity", "facingEntity"] => teleport_to_pos(
            run,
            parse,
            source,
            cx,
            TeleportTargets::Argument,
            false,
            Some(FacingShape::Entity),
        ),
        ["teleport", "targets", "location", "facing", "entity", "facingEntity", "facingAnchor"] => {
            teleport_to_pos(
                run,
                parse,
                source,
                cx,
                TeleportTargets::Argument,
                false,
                Some(FacingShape::AnchoredEntity),
            )
        }
        ["teleport", "targets", "location", "facing", "facingLocation"] => teleport_to_pos(
            run,
            parse,
            source,
            cx,
            TeleportTargets::Argument,
            false,
            Some(FacingShape::Position),
        ),
        // SpreadPlayersCommand.java:60-69 (default maxHeight = getMaxY()+1)
        // and `:74-90` (`under <maxHeight>`).
        ["spreadplayers", "center", "spreadDistance", "maxRange", "respectTeams", "targets"] => {
            spread_players(run, parse, source, cx, None)
        }
        ["spreadplayers", "center", "spreadDistance", "maxRange", "under", "maxHeight", "respectTeams", "targets"] => {
            spread_players(
                run,
                parse,
                source,
                cx,
                Some(integer_argument(parse, "maxHeight")),
            )
        }
        ["dialog", "show", "targets", "dialog"] => show_dialog(run, parse, source, cx),
        ["dialog", "clear", "targets"] => clear_dialog(run, parse, source, cx),
        // The six `/title` executors (TitleCommand.java:32-33, `:38-46`,
        // `:54-61`, `:69-76`, `:88-94`): clear/reset send one packet each,
        // the three show literals share `showTitle`, and `times` is `setTimes`.
        ["title", "targets", "clear"] => clear_title(run, parse, source, cx),
        ["title", "targets", "reset"] => reset_title(run, parse, source, cx),
        ["title", "targets", "title", "title"] => show_title(run, parse, source, cx, "title"),
        ["title", "targets", "subtitle", "title"] => show_title(run, parse, source, cx, "subtitle"),
        ["title", "targets", "actionbar", "title"] => {
            show_title(run, parse, source, cx, "actionbar")
        }
        ["title", "targets", "times", "fadeIn", "stay", "fadeOut"] => {
            set_title_times(run, parse, source, cx)
        }
        ["particle", "name"]
        | ["particle", "name", "pos"]
        | ["particle", "name", "pos", "delta", "speed", "count"]
        | ["particle", "name", "pos", "delta", "speed", "count", "normal"]
        | ["particle", "name", "pos", "delta", "speed", "count", "normal", "viewers"] => {
            send_particles(run, parse, source, cx, false)
        }
        ["particle", "name", "pos", "delta", "speed", "count", "force"]
        | ["particle", "name", "pos", "delta", "speed", "count", "force", "viewers"] => {
            send_particles(run, parse, source, cx, true)
        }
        // The sixty-one `/playsound` executors: one on the bare `<sound>`
        // (PlaySoundCommand.java:36-47), which is `MASTER` over the calling
        // player, and six per source literal (`:56-145`) that differ only in
        // how many of the optional arguments were typed. The literal's own
        // executor (`:58-69`) still sends to the calling player alone; the
        // `<targets>` node and everything under it send to the targets.
        ["playsound", "sound"] => play_sound(run, parse, source, cx, None),
        ["playsound", "sound", sound_source]
        | ["playsound", "sound", sound_source, "targets"]
        | ["playsound", "sound", sound_source, "targets", "pos"]
        | ["playsound", "sound", sound_source, "targets", "pos", "volume"]
        | ["playsound", "sound", sound_source, "targets", "pos", "volume", "pitch"]
        | ["playsound", "sound", sound_source, "targets", "pos", "volume", "pitch", "minVolume"] => {
            play_sound(run, parse, source, cx, Some(sound_source))
        }
        // The twenty-three `/stopsound` executors: the bare `<targets>` node
        // (StopSoundCommand.java:23), the `*`-literal any-source form with a
        // named sound (`:26-31`), and two per source literal (`:35-42`).
        ["stopsound", "targets"] | ["stopsound", "targets", "*", "sound"] => {
            stop_sound(run, parse, source, cx, None)
        }
        ["stopsound", "targets", sound_source]
        | ["stopsound", "targets", sound_source, "sound"] => {
            stop_sound(run, parse, source, cx, Some(sound_source))
        }
        ["loot", ..] => drop_loot(run, parse, source, cx),
        ["give", "targets", "item"] | ["give", "targets", "item", "count"] => {
            give_items(run, parse, source, cx)
        }
        ["item", "replace", "block", "pos", "slot", "with", "item"]
        | ["item", "replace", "block", "pos", "slot", "with", "item", "count"] => {
            set_block_item(run, parse, source, cx)
        }
        ["item", "replace", "entity", "targets", "slot", "with", "item"]
        | ["item", "replace", "entity", "targets", "slot", "with", "item", "count"] => {
            set_entity_item(run, parse, source, cx)
        }
        // Each `from` arm has a second `executes` under a `<modifier>` child
        // (`ItemCommands.java:114-126`, `:145-157`, `:213-225`, `:244-256`);
        // the body is the same one with `applyModifier` between the read and
        // the write, so the arms are paired here.
        ["item", "replace", "block", "pos", "slot", "from", "block", "source", "sourceSlot"]
        | ["item", "replace", "block", "pos", "slot", "from", "block", "source", "sourceSlot", "modifier"] => {
            block_to_block(run, parse, source, cx)
        }
        ["item", "replace", "entity", "targets", "slot", "from", "block", "source", "sourceSlot"]
        | ["item", "replace", "entity", "targets", "slot", "from", "block", "source", "sourceSlot", "modifier"] => {
            block_to_entities(run, parse, source, cx)
        }
        ["item", "replace", "block", "pos", "slot", "from", "entity", "source", "sourceSlot"]
        | ["item", "replace", "block", "pos", "slot", "from", "entity", "source", "sourceSlot", "modifier"] => {
            entity_to_block(run, parse, source, cx)
        }
        ["item", "replace", "entity", "targets", "slot", "from", "entity", "source", "sourceSlot"]
        | ["item", "replace", "entity", "targets", "slot", "from", "entity", "source", "sourceSlot", "modifier"] => {
            entity_to_entities(run, parse, source, cx)
        }
        ["item", "modify", "block", "pos", "slot", "modifier"] => {
            modify_block_item(run, parse, source, cx)
        }
        ["item", "modify", "entity", "targets", "slot", "modifier"] => {
            modify_entity_item(run, parse, source, cx)
        }
        ["clear"] => clear_items(run, parse, source, cx, false),
        ["clear", "targets"]
        | ["clear", "targets", "item"]
        | ["clear", "targets", "item", "maxCount"] => clear_items(run, parse, source, cx, true),
        ["effect", "clear"] => clear_effects(run, parse, source, cx, true),
        ["effect", "clear", "targets"] | ["effect", "clear", "targets", "effect"] => {
            clear_effects(run, parse, source, cx, false)
        }
        ["effect", "give", "targets", "effect", "seconds"]
        | ["effect", "give", "targets", "effect", "seconds", "amplifier"]
        | ["effect", "give", "targets", "effect", "seconds", "amplifier", "hideParticles"] => {
            let seconds = integer_argument(parse, "seconds");
            give_effect(run, parse, source, cx, Some(seconds))
        }
        ["effect", "give", "targets", "effect"] => give_effect(run, parse, source, cx, None),
        ["effect", "give", "targets", "effect", "infinite"]
        | ["effect", "give", "targets", "effect", "infinite", "amplifier"]
        | ["effect", "give", "targets", "effect", "infinite", "amplifier", "hideParticles"] => {
            give_effect(run, parse, source, cx, Some(-1))
        }
        // The five `/damage` executors (DamageCommand.java:29-36, :39-46,
        // :51-60, :67-76 and :81-92), which differ only in the `DamageSource`
        // they build.
        ["damage", "target", "amount"] => damage_entity(run, parse, source, cx, None),
        ["damage", "target", "amount", "damageType"] => {
            damage_entity(run, parse, source, cx, Some(DamageSourceShape::Type))
        }
        ["damage", "target", "amount", "damageType", "at", "location"] => {
            damage_entity(run, parse, source, cx, Some(DamageSourceShape::At))
        }
        ["damage", "target", "amount", "damageType", "by", "entity"] => {
            damage_entity(run, parse, source, cx, Some(DamageSourceShape::By))
        }
        ["damage", "target", "amount", "damageType", "by", "entity", "from", "cause"] => {
            damage_entity(run, parse, source, cx, Some(DamageSourceShape::ByFrom))
        }
        ["datapack", "enable", "name"] => enable_pack(run, parse, cx, PackInsertion::Default),
        ["datapack", "enable", "name", "after", "existing"] => {
            enable_pack(run, parse, cx, PackInsertion::After)
        }
        ["datapack", "enable", "name", "before", "existing"] => {
            enable_pack(run, parse, cx, PackInsertion::Before)
        }
        ["datapack", "enable", "name", "last"] => enable_pack(run, parse, cx, PackInsertion::Last),
        ["datapack", "enable", "name", "first"] => {
            enable_pack(run, parse, cx, PackInsertion::First)
        }
        ["datapack", "disable", "name"] => disable_pack(run, parse, cx),
        ["reload"] => Ok(reload(run, cx)),
        ["msg", "targets", "message"] => message_players(run, parse, source, cx),
        ["whitelist", "reload"] => Ok(reload_whitelist(run, cx)),
        ["save-all"] => Ok(save_all(run, cx, false)),
        ["save-all", "flush"] => Ok(save_all(run, cx, true)),
        ["stop"] => Ok(stop(run, cx)),
        _ => {
            let result = dispatch_rest(run, parse, source, cx, &path);
            // Scoreboard writers change the live board; what it queued goes
            // out whether or not the command went on to throw.
            cx.world.flush_command_scoreboard(cx.events);
            result
        }
    }
}

fn weather_kind(literal: &str) -> CommandWeatherKind {
    match literal {
        "clear" => CommandWeatherKind::Clear,
        "rain" => CommandWeatherKind::Rain,
        "thunder" => CommandWeatherKind::Thunder,
        other => unreachable!("/weather has no {other:?} literal"),
    }
}

fn time_arm<'p>(rest: &'p [&'p str]) -> &'p [&'p str] {
    if matches!(rest, ["of", ..]) {
        &rest[2..]
    } else {
        rest
    }
}

fn dispatch_rest(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
    path: &[&str],
) -> Outcome {
    let at = &cx.origin;
    let world: &mut ServerWorld = cx.world;
    let node = parse
        .executable_node()
        .expect("only a parse that reached a command is run");
    match path {
        ["list"] => Ok(list_players(run, world, false)),
        ["list", "uuids"] => Ok(list_players(run, world, true)),
        ["datapack", "list"] => Ok(list_packs(run, world)),
        ["datapack", "list", "available"] => Ok(list_available_packs(run, world)),
        ["datapack", "list", "enabled"] => Ok(list_enabled_packs(run, world)),
        // `DataPackCommand.register` (DataPackCommand.java:102-133) hangs one
        // `Inserter` on each of the five enable leaves.
        // `ScoreboardCommand.register` (ScoreboardCommand.java:68-369): the
        // `objectives` half.
        ["scoreboard", "objectives", "list"] => list_objectives(run, world),
        ["scoreboard", "objectives", "add", "objective", "criteria"] => {
            add_objective(run, parse, source, cx.world, at, false)
        }
        ["scoreboard", "objectives", "add", "objective", "criteria", "displayName"] => {
            add_objective(run, parse, source, cx.world, at, true)
        }
        ["scoreboard", "objectives", "remove", "objective"] => {
            remove_objective(run, parse, cx.world)
        }
        ["scoreboard", "objectives", "modify", "objective", "displayname", "displayName"] => {
            set_objective_display_name(run, parse, source, cx.world, at)
        }
        ["scoreboard", "objectives", "modify", "objective", "rendertype", name] => {
            set_objective_render_type(
                run,
                parse,
                cx.world,
                if *name == "hearts" {
                    ObjectiveRenderType::Hearts
                } else {
                    ObjectiveRenderType::Integer
                },
            )
        }
        ["scoreboard", "objectives", "modify", "objective", "displayautoupdate", "value"] => {
            set_objective_display_auto_update(run, parse, cx.world)
        }
        ["scoreboard", "objectives", "modify", "objective", "numberformat"] => {
            set_objective_number_format(run, parse, source, cx.world, at, NumberFormatArm::Clear)
        }
        ["scoreboard", "objectives", "modify", "objective", "numberformat", "blank"] => {
            set_objective_number_format(run, parse, source, cx.world, at, NumberFormatArm::Blank)
        }
        ["scoreboard", "objectives", "modify", "objective", "numberformat", "fixed", "contents"] => {
            set_objective_number_format(run, parse, source, cx.world, at, NumberFormatArm::Fixed)
        }
        ["scoreboard", "objectives", "modify", "objective", "numberformat", "styled", "style"] => {
            set_objective_number_format(run, parse, source, cx.world, at, NumberFormatArm::Styled)
        }
        ["scoreboard", "objectives", "setdisplay", "slot"] => {
            clear_display_slot(run, parse, cx.world)
        }
        ["scoreboard", "objectives", "setdisplay", "slot", "objective"] => {
            set_display_slot(run, parse, cx.world)
        }
        // The `players` half (`:164-367`).
        ["scoreboard", "players", "list"] => list_tracked_players(run, world),
        ["scoreboard", "players", "list", "target"] => {
            list_tracked_player_scores(run, parse, source, world, at)
        }
        ["scoreboard", "players", "get", "target", "objective"] => {
            get_score(run, parse, source, world, at)
        }
        ["scoreboard", "players", "set", "targets", "objective", "score"] => {
            set_score(run, parse, source, cx.world, at)
        }
        ["scoreboard", "players", "add", "targets", "objective", "score"] => {
            add_score(run, parse, source, cx.world, at, false)
        }
        ["scoreboard", "players", "remove", "targets", "objective", "score"] => {
            add_score(run, parse, source, cx.world, at, true)
        }
        ["scoreboard", "players", "reset", "targets"] => {
            reset_all_scores(run, parse, source, cx.world, at)
        }
        ["scoreboard", "players", "reset", "targets", "objective"] => {
            reset_score(run, parse, source, cx.world, at)
        }
        ["scoreboard", "players", "enable", "targets", "objective"] => {
            enable_trigger(run, parse, source, cx.world, at)
        }
        ["scoreboard", "players", "display", "name", "targets", "objective"] => {
            set_score_display(run, parse, source, cx.world, at, false)
        }
        ["scoreboard", "players", "display", "name", "targets", "objective", "name"] => {
            set_score_display(run, parse, source, cx.world, at, true)
        }
        ["scoreboard", "players", "display", "numberformat", "targets", "objective"] => {
            set_score_number_format(run, parse, source, cx.world, at, NumberFormatArm::Clear)
        }
        ["scoreboard", "players", "display", "numberformat", "targets", "objective", "blank"] => {
            set_score_number_format(run, parse, source, cx.world, at, NumberFormatArm::Blank)
        }
        ["scoreboard", "players", "display", "numberformat", "targets", "objective", "fixed", "contents"] => {
            set_score_number_format(run, parse, source, cx.world, at, NumberFormatArm::Fixed)
        }
        ["scoreboard", "players", "display", "numberformat", "targets", "objective", "styled", "style"] => {
            set_score_number_format(run, parse, source, cx.world, at, NumberFormatArm::Styled)
        }
        ["scoreboard", "players", "operation", "targets", "targetObjective", "operation", "source", "sourceObjective"] => {
            perform_operation(run, parse, source, cx.world, at)
        }
        ["say", "message"] => {
            chat::broadcast(run, parse, "message", source, cx, ChatKind::SayCommand)
        }
        ["me", "action"] => {
            chat::broadcast(run, parse, "action", source, cx, ChatKind::EmoteCommand)
        }
        ["seed"] => Ok(seed(run, world)),
        // AttributeCommand.java:51-66 and AttributeCommand.java:86-102.
        ["attribute", "target", "attribute", "get"] => {
            get_attribute_value(run, parse, source, world, at, false, false)
        }
        ["attribute", "target", "attribute", "get", "scale"] => {
            get_attribute_value(run, parse, source, world, at, true, false)
        }
        ["attribute", "target", "attribute", "base", "get"] => {
            get_attribute_value(run, parse, source, world, at, false, true)
        }
        ["attribute", "target", "attribute", "base", "get", "scale"] => {
            get_attribute_value(run, parse, source, world, at, true, true)
        }
        ["attribute", "target", "attribute", "modifier", "value", "get", "id"] => {
            get_attribute_modifier_value(run, parse, source, world, at, false)
        }
        ["attribute", "target", "attribute", "modifier", "value", "get", "id", "scale"] => {
            get_attribute_modifier_value(run, parse, source, world, at, true)
        }
        ["forceload", "query"] => list_force_load(run, world, at),
        ["forceload", "query", "pos"] => query_force_load(run, parse, world, at),
        ["difficulty"] => Ok(query_difficulty(run, world)),
        ["gamerule", rule] => query_game_rule(run, rule, world),
        ["help"] | ["help", "command"] => help(run, parse, source, world),
        ["banlist"] => Ok(ban_list(run, world, true, true)),
        ["banlist", "ips"] => Ok(ban_list(run, world, false, true)),
        ["banlist", "players"] => Ok(ban_list(run, world, true, false)),
        ["whitelist", "list"] => Ok(whitelist_list(run, world)),
        ["stopwatch", "query", "id"] => query_stopwatch(run, parse, world, 1.0),
        ["stopwatch", "query", "id", "scale"] => {
            query_stopwatch(run, parse, world, double_argument(parse, "scale"))
        }
        ["tick", "query"] => Ok(tick_query(run, world)),
        // `TimeCommand.addClockNodes` (`TimeCommand.java:64-112`) hangs the same
        // six arms off the bare `/time` root and off `of <clock>`, so which of
        // the two the path took is the whole difference between them: it
        // decides only which clock the arm runs on (`:48`, `:57`).
        ["time", "query", "gametime"] => Ok(time::query_game_time(run, world)),
        ["time", rest @ ..] => {
            let explicit = matches!(rest, ["of", ..]);
            let arm = if explicit { &rest[2..] } else { rest };
            match arm {
                ["query", "time"] => time::query_clock_time(run, parse, world, at, explicit),
                ["query", "timeline"] => {
                    time::query_timeline(run, parse, world, at, explicit, false)
                }
                ["query", "timeline", "repetition"] => {
                    time::query_timeline(run, parse, world, at, explicit, true)
                }
                other => unreachable!("/time has no {other:?} arm"),
            }
        }
        // The twenty-seven `/team` executors (TeamCommand.java:63-229). The
        // twelve visibility and collision arms are literals rather than an
        // argument, so each names its own enum constant.
        ["team", "list"] => list_teams(run, world),
        ["team", "list", "team"] => list_team_members(run, parse, world),
        ["team", "add", "team"] => add_team(run, parse, source, cx.world, at, false),
        ["team", "add", "team", "displayName"] => add_team(run, parse, source, cx.world, at, true),
        ["team", "remove", "team"] => remove_team(run, parse, cx.world),
        ["team", "empty", "team"] => empty_team(run, parse, cx.world),
        ["team", "join", "team"] => join_team(run, parse, source, cx.world, at, false),
        ["team", "join", "team", "members"] => join_team(run, parse, source, cx.world, at, true),
        ["team", "leave", "members"] => leave_team(run, parse, source, cx.world, at),
        ["team", "modify", "team", "displayName", "displayName"] => {
            set_team_display_name(run, parse, source, cx.world, at)
        }
        ["team", "modify", "team", "color", "value"] => set_team_color(run, parse, cx.world),
        ["team", "modify", "team", "friendlyFire", "allowed"] => {
            set_team_flag(run, parse, cx.world, false)
        }
        ["team", "modify", "team", "seeFriendlyInvisibles", "allowed"] => {
            set_team_flag(run, parse, cx.world, true)
        }
        ["team", "modify", "team", "nametagVisibility", visibility] => {
            set_team_visibility(run, parse, cx.world, team_visibility(visibility), false)
        }
        ["team", "modify", "team", "deathMessageVisibility", visibility] => {
            set_team_visibility(run, parse, cx.world, team_visibility(visibility), true)
        }
        ["team", "modify", "team", "collisionRule", rule] => {
            set_team_collision(run, parse, cx.world, team_collision_rule(rule))
        }
        ["team", "modify", "team", "prefix", "prefix"] => {
            set_team_affix(run, parse, source, cx.world, at, false)
        }
        ["team", "modify", "team", "suffix", "suffix"] => {
            set_team_affix(run, parse, source, cx.world, at, true)
        }
        // The three `/trigger` executors (TriggerCommand.java:37, `:44-48`,
        // `:58-62`).
        ["trigger", "objective"] => trigger(run, parse, source, cx.world, TriggerArm::Simple),
        ["trigger", "objective", "add", "value"] => {
            trigger(run, parse, source, cx.world, TriggerArm::Add)
        }
        ["trigger", "objective", "set", "value"] => {
            trigger(run, parse, source, cx.world, TriggerArm::Set)
        }
        // The two `/ride` executors (RideCommand.java:45, `:48`).
        ["ride", "target", "mount", "vehicle"] => mount(run, parse, source, cx),
        ["ride", "target", "dismount"] => dismount(run, parse, source, cx),
        ["tag", "targets", "list"] => list_tags(run, parse, source, world, at),
        ["version"] => Ok(report_version(run)),
        ["experience", "query", "target", "points"] => {
            query_experience(run, parse, source, world, at, CommandExperienceKind::Points)
        }
        ["experience", "query", "target", "levels"] => {
            query_experience(run, parse, source, world, at, CommandExperienceKind::Levels)
        }
        ["waypoint", "list"] => Ok(list_waypoints(run, world, at)),
        ["worldborder", "get"] => get_world_border_size(run, world, at),
        // The published `if`/`unless` arms, whose own node is executable as
        // well as forking (`ExecuteCommand.java:883-890`).
        ["execute", conditional @ ("if" | "unless"), "block", "pos", "block"]
        | ["execute", conditional @ ("if" | "unless"), "dimension", "dimension"]
        | ["execute", conditional @ ("if" | "unless"), "entity", "entities"]
        | ["execute", conditional @ ("if" | "unless"), "predicate", "predicate"]
        | ["execute", conditional @ ("if" | "unless"), "items", "entity" | "block", _, "slots", "item_predicate"]
        | ["execute", conditional @ ("if" | "unless"), "loaded", "pos"]
        | ["execute", conditional @ ("if" | "unless"), "biome", "pos", "biome"]
        | ["execute", conditional @ ("if" | "unless"), "stopwatch", "id", "range"]
        | ["execute", conditional @ ("if" | "unless"), "score", "target", "targetObjective", "matches", "range"]
        | ["execute", conditional @ ("if" | "unless"), "score", "target", "targetObjective", _, "source", "sourceObjective"]
        | ["execute", conditional @ ("if" | "unless"), "data", "block" | "entity" | "storage", _, "path"]
        | ["execute", conditional @ ("if" | "unless"), "blocks", "start", "end", "destination", "all" | "masked"] => {
            conditional_leaf(run, parse, node, *conditional == "if", source, cx.world, at)
        }
        other => unreachable!("no executor for the command path {other:?}"),
    }
}

/// The `CustomCommandExecutor` nodes this server publishes.
///
/// `BuildContexts` runs one itself (BuildContexts.java:102-107) rather than
/// scheduling an `ExecuteCommand` for it, so none of them spends command quota
/// and none of them produces an int: what a caller eventually sees comes from
/// the frame each one reaches through its [`ExecutionControl`].
pub(super) fn run_custom(
    node: u16,
    parse: &ParseResult,
    source: &Arc<ExecutionSource>,
    is_return: bool,
    forked: bool,
    control: &mut ExecutionControl<'_, '_>,
) -> CommandRun {
    let mut run = CommandRun::with_registry(control.world().plugins().content_registry());
    let outcome = (|| match catalog::canonical_path(node).as_slice() {
        path @ (["function", "name"]
        | ["function", "name", "arguments"]
        | ["function", "name", "with", "entity", "source"]
        | ["function", "name", "with", "entity", "source", "path"]
        | ["function", "name", "with", "block", "sourcePos"]
        | ["function", "name", "with", "block", "sourcePos", "path"]
        | ["function", "name", "with", "storage", "source"]
        | ["function", "name", "with", "storage", "source", "path"]) => {
            call_function(&mut run, path, parse, source, is_return, control)
        }
        ["debug", "function", "name"] => {
            let (tag, name) = match parse.argument("name") {
                Some(ArgumentValue::Function { tag, id }) => (*tag, id.as_str()),
                _ => unreachable!("the function argument was parsed"),
            };
            // DebugCommand.java:109-118 checks tracing guards before resolving functions.
            control.check_trace_allowed(is_return)?;
            let functions = resolve_functions(tag, name, control.world())?;
            run.absorb(control.trace_functions(functions, source));
            Ok(())
        }
        ["return", "value"] => {
            return_value(parse, source, control);
            Ok(())
        }
        ["return", "fail"] => {
            return_fail(source, control);
            Ok(())
        }
        other => unreachable!("no custom executor for the command path {other:?}"),
    })();
    if let Err(message) = outcome {
        // `CustomCommandExecutor.WithErrorHandling.run`
        // (CustomCommandExecutor.java:20-28): `handleError` first, silent in
        // a forked chain (`:30-32`), then the source's own callback, which an
        // enclosing `/execute store` reads as the zero `onFailure` carries.
        if !forked {
            run.fail(message);
        } else {
            run.trace_errors.push((run.feedback.len(), message));
        }
        control.source_result(source, false, 0);
    }
    run
}

/// A single entity resolved from players and the native non-player store.
pub(super) fn single_damage_entity_argument<'a>(
    parse: &ParseResult,
    name: &str,
    source: &CommandSource,
    at: &CommandOrigin,
    world: &EntityReads<'a>,
) -> Result<CommandDamageEntityRef<'a>, NbtSummaryValue> {
    let Some(ArgumentValue::Players(selector)) = parse.argument(name) else {
        unreachable!("{name} is an entity argument");
    };
    single_damage_entity_of(selector, source, at, world)
}

/// [`single_damage_entity_argument`] for a selector a caller already read,
/// which is what a forked `/execute if data entity` carries out of its own
/// node.
pub(super) fn single_damage_entity_of<'a>(
    selector: &Selector,
    source: &CommandSource,
    at: &CommandOrigin,
    world: &EntityReads<'a>,
) -> Result<CommandDamageEntityRef<'a>, NbtSummaryValue> {
    let found = select_damage_entity_states(selector, source, at, world)?;
    let Some(first) = found.first().copied() else {
        return Err(translatable("argument.entity.notfound.entity", Vec::new()));
    };
    if found.len() > 1 {
        return Err(translatable("argument.entity.toomany", Vec::new()));
    }
    Ok(first)
}

/// `EntityArgument.getOptionalEntities` (`EntityArgument.java:69-71`): empty
/// is a zero-length list, not a refusal.
pub(super) fn optional_command_entities<'a>(
    selector: &Selector,
    source: &CommandSource,
    at: &CommandOrigin,
    world: &EntityReads<'a>,
) -> Result<Vec<CommandDamageEntityRef<'a>>, NbtSummaryValue> {
    select_damage_entity_states(selector, source, at, world)
}

pub(super) fn command_entity_source(
    entity: CommandDamageEntityRef<'_>,
    permission: CommandPermission,
    world: &EntityReads<'_>,
) -> CommandSource {
    entity.as_command_source_named(permission, world)
}

/// The `getEyeHeight` an `EYES` anchor reads off `CommandSourceStack.entity`
/// (`EntityAnchorArgument.java:85-88`), which changes with the entity every
/// `withEntity` binds. `None` is a source with no entity, which that same body
/// answers with the untransformed position.
pub(super) fn source_eye_height(source: &CommandSource, world: &EntityReads<'_>) -> Option<f32> {
    source_entity_state(source, world).map(CommandDamageEntityRef::eye_height)
}

/// What one published `if`/`unless` arm answers
/// (`ExecuteCommand.java:471-483`, `:501-521`, `:632-639`).
///
/// `None` is the arm answering no: an unmet predicate, a zero count, or the
/// empty `OptionalInt` `checkRegions` returns (`:920-992`). `Some` carries the
/// count the two numeric handlers report, which for the region arm may be
/// zero.
///
/// The block arm resolves its position through `getLoadedBlockPos`
/// (BlockPosArgument.java:37-46), which is where an unloaded or out-of-world
/// column refuses; the loaded arm uses `getBlockPos` and answers false
/// instead (`ExecuteCommand.java:507`); the dimension arm is the same
/// `getDimension` lookup `/execute in` performs, compared against the
/// source's own level.
pub(super) fn execute_condition_count(
    condition: &ExecuteCondition,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
) -> Result<Option<i32>, NbtSummaryValue> {
    let blocks = world.command_blocks(&at.level);
    match condition {
        ExecuteCondition::Block { pos, predicate } => {
            let pos = blocks::loaded_block_pos_of(*pos, blocks, at)?;
            // `ExecuteCommand.java:471-483` builds the same `BlockInWorld` the
            // two filter forms do, so a predicate that names a compound is
            // compared against the block entity at the position as well.
            let resolver = world.item_stack_resolver();
            Ok(held(blocks::block_predicate_matches(
                predicate,
                blocks.block_state_at(pos),
                blocks::block_entity_data_at(blocks, pos, Some(resolver.as_dyn()))?.as_ref(),
                world.tags(),
            )))
        }
        ExecuteCondition::Dimension(id) => {
            let level = super::parse::apply_execute_in(id, world, at.clone())?.level;
            Ok(held(level == at.level))
        }
        ExecuteCondition::Entity(selector) => {
            let count = world.with_entity_reads(|reads| {
                optional_command_entities(selector, source, at, reads)
                    .map(|entities| entities.len())
            })?;
            Ok(counted(i32::try_from(count).unwrap_or(i32::MAX)))
        }
        ExecuteCondition::Predicate(predicate) => world
            .with_entity_reads(|reads| loot::predicate_holds(predicate, source, reads, at))
            .map(held),
        ExecuteCondition::Items {
            source: input,
            slots,
            predicate,
        } => Ok(counted(item::count_items(
            input, slots, predicate, source, world, at,
        )?)),
        ExecuteCondition::Loaded { pos } => Ok(held(
            world.is_execute_chunk_loaded(&at.level, pos.block_pos(at)),
        )),
        ExecuteCondition::Biome { pos, tag, id } => {
            let pos = blocks::loaded_block_pos_of(*pos, blocks, at)?;
            let Some(biome) = blocks
                .biome_id_at(pos.x, pos.y, pos.z)
                .and_then(|biome| world.registry_element_id("minecraft:worldgen/biome", biome))
            else {
                return Ok(None);
            };
            Ok(held(if *tag {
                world
                    .biome_tags()
                    .is_some_and(|tags| tags.contains("minecraft:worldgen/biome", id, biome))
            } else {
                biome == id
            }))
        }
        ExecuteCondition::Score {
            target,
            objective,
            test,
        } => {
            // `checkScore` resolves both holders and both objectives before it
            // reads either score (`ExecuteCommand.java:831-837`), so a missing
            // objective refuses where a missing score only answers false.
            let target = scoreboard::single_holder_of(target, source, world, at, Wildcard::Empty)?;
            let objective = scoreboard::objective_of(objective, world)?;
            let matched = match test {
                ScoreTest::Compare {
                    operation,
                    source: holder,
                    objective: source_objective,
                } => {
                    let holder =
                        scoreboard::single_holder_of(holder, source, world, at, Wildcard::Empty)?;
                    let source_objective = scoreboard::objective_of(source_objective, world)?;
                    let board = world.scoreboard();
                    match (
                        board.score(&target.name, &objective.name),
                        board.score(&holder.name, &source_objective.name),
                    ) {
                        (Some(a), Some(b)) => operation.holds(a.value, b.value),
                        _ => false,
                    }
                }
                ScoreTest::Matches { min, max } => world
                    .scoreboard()
                    .score(&target.name, &objective.name)
                    .is_some_and(|score| {
                        min.is_none_or(|min| score.value >= min)
                            && max.is_none_or(|max| score.value <= max)
                    }),
            };
            Ok(held(matched))
        }
        ExecuteCondition::Data { provider, path } => Ok(counted(data::matching_data(
            provider, path, source, world, at,
        )?)),
        ExecuteCondition::Blocks {
            start,
            end,
            destination,
            skip_air,
        } => blocks::check_regions(*start, *end, *destination, *skip_air, world, at),
        ExecuteCondition::Stopwatch { id, min, max } => {
            let Some(stopwatch) = world.stopwatch(id) else {
                return Err(translatable(
                    "commands.stopwatch.does_not_exist",
                    vec![component_literal_nbt(id)],
                ));
            };
            let elapsed = stopwatch.elapsed_seconds(crate::world::idle_timeout::monotonic_millis());
            let matches =
                min.is_none_or(|min| elapsed >= min) && max.is_none_or(|max| elapsed <= max);
            Ok(held(matches))
        }
    }
}

/// A yes-or-no arm's answer: `addConditional` reads only whether the predicate
/// held (`ExecuteCommand.java:883`), so the count it never reports is one.
fn held(matched: bool) -> Option<i32> {
    matched.then_some(1)
}

/// A counting arm's answer, where `createNumericConditionalHandler` treats a
/// zero as the condition failing (`ExecuteCommand.java:809`, `:817`).
fn counted(count: i32) -> Option<i32> {
    (count > 0).then_some(count)
}

/// `expect`'s own question (`ExecuteCommand.java:873-875`), which for the
/// counting arms is `!getOptionalEntities(..).isEmpty()` (`:636`) and for the
/// region arm `checkRegions(..).isPresent()` (`:896`).
pub(super) fn execute_condition_holds(
    condition: &ExecuteCondition,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
) -> Result<bool, NbtSummaryValue> {
    Ok(execute_condition_count(condition, source, world, at)?.is_some())
}

/// What one `/execute store` arm resolves to while its modifier runs
/// (`wrapStores`, `ExecuteCommand.java:277-303`).
pub(super) fn resolve_execute_store(
    arm: &ExecuteStoreArm,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
) -> Result<StoreTarget, NbtSummaryValue> {
    match arm {
        ExecuteStoreArm::Score { targets, objective } => {
            scoreboard::resolve_store_score(targets, objective, source, world, at)
        }
        ExecuteStoreArm::BossBar { id, into_value } => {
            bossbar::resolve_store_boss_bar(id, *into_value, world)
        }
        ExecuteStoreArm::Data {
            provider,
            path,
            tag,
            scale,
        } => {
            let target = data::store_accessor(provider, source, world, at)?;
            let instance = if let Some(super::CommandDataTarget::Block { level, pos }) = &target {
                world
                    .command_blocks(level)
                    .block_entities()
                    .get(*pos)
                    .cloned()
            } else {
                None
            };
            Ok(StoreTarget::Data {
                target,
                instance: instance.map(Box::new),
                path: path.clone(),
                tag: *tag,
                scale: *scale,
            })
        }
    }
}

/// `addConditional`'s `.executes` (`ExecuteCommand.java:883-890`): the arm that
/// ends a chain reports the pass itself and refuses with
/// `commands.execute.conditional.fail`.
fn conditional_leaf(
    run: &mut CommandRun,
    parse: &ParseResult,
    node: u16,
    expected: bool,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
) -> Result<i32, NbtSummaryValue> {
    // The executable conditional is the parse's last node.
    let index = parse
        .nodes
        .iter()
        .rposition(|parsed| parsed.index == node)
        .expect("the executable node is in the parse");
    let condition = super::parse::execute_condition(parse, index);
    let count = execute_condition_count(&condition, source, world, at)?;
    if !matches!(
        condition,
        ExecuteCondition::Entity(_)
            | ExecuteCondition::Items { .. }
            | ExecuteCondition::Data { .. }
            | ExecuteCondition::Blocks { .. }
    ) {
        // `addConditional`'s own handler (`ExecuteCommand.java:883-890`).
        return if count.is_some() == expected {
            run.succeed(
                translatable("commands.execute.conditional.pass", Vec::new()),
                false,
            );
            Ok(1)
        } else {
            Err(translatable(
                "commands.execute.conditional.fail",
                Vec::new(),
            ))
        };
    }
    // `createNumericConditionalHandler` (`ExecuteCommand.java:806-824`), which
    // `checkIfRegions` / `checkUnlessRegions` (`:900-918`) repeat over the
    // `OptionalInt`: `if` reports and returns the count itself, `unless` only
    // passes on nothing found and otherwise names the count it found.
    match (expected, count) {
        (true, Some(count)) => {
            run.succeed(
                translatable(
                    "commands.execute.conditional.pass_count",
                    vec![NbtSummaryValue::Int(count)],
                ),
                false,
            );
            Ok(count)
        }
        (true, None) => Err(translatable(
            "commands.execute.conditional.fail",
            Vec::new(),
        )),
        (false, None) => {
            run.succeed(
                translatable("commands.execute.conditional.pass", Vec::new()),
                false,
            );
            Ok(1)
        }
        (false, Some(count)) => Err(translatable(
            "commands.execute.conditional.fail_count",
            vec![NbtSummaryValue::Int(count)],
        )),
    }
}

pub(super) fn apply_execute_positioned_as_entity(
    entity: CommandDamageEntityRef<'_>,
    at: CommandOrigin,
) -> CommandOrigin {
    at.with_position(entity.position())
}

pub(super) fn apply_execute_rotated_as_entity(
    entity: CommandDamageEntityRef<'_>,
    at: CommandOrigin,
) -> CommandOrigin {
    at.with_rotation(entity.rotation())
}

pub(super) fn apply_execute_facing_entity(
    entity: CommandDamageEntityRef<'_>,
    anchor: EntityAnchor,
    at: CommandOrigin,
) -> CommandOrigin {
    at.facing(entity.anchored_position(anchor))
}

/// `EntityArgument.getPlayer` (EntityArgument.java:81-83) through
/// `EntitySelector.findSinglePlayer`
/// (commands/arguments/selector/EntitySelector.java:197-205), whose refusal is
/// the not-found key for any count other than exactly one.
fn single_player_argument<'a>(
    parse: &ParseResult,
    name: &str,
    source: &CommandSource,
    world: &EntityReads<'a>,
    at: &CommandOrigin,
) -> Result<CommandPlayer<'a>, NbtSummaryValue> {
    let Some(ArgumentValue::Players(selector)) = parse.argument(name) else {
        unreachable!("{name} is an entity argument");
    };
    let found = select_player_states(selector, source, at, world)?;
    if found.len() != 1 {
        // `:200-201`.
        return Err(translatable("argument.entity.notfound.player", Vec::new()));
    }
    Ok(found[0])
}

/// `EntityArgument.getEntities` (EntityArgument.java:60-67).
pub(super) fn damage_entity_targets<'a>(
    parse: &ParseResult,
    name: &str,
    source: &CommandSource,
    at: &CommandOrigin,
    world: &EntityReads<'a>,
) -> Result<Vec<CommandDamageEntityRef<'a>>, NbtSummaryValue> {
    let Some(ArgumentValue::Players(selector)) = parse.argument(name) else {
        unreachable!("{name} is an entity argument");
    };
    required_command_entities(selector, source, at, world)
}

pub(super) fn required_command_entities<'a>(
    selector: &Selector,
    source: &CommandSource,
    at: &CommandOrigin,
    world: &EntityReads<'a>,
) -> Result<Vec<CommandDamageEntityRef<'a>>, NbtSummaryValue> {
    let found = select_damage_entity_states(selector, source, at, world)?;
    if found.is_empty() {
        // `:63-64`.
        return Err(translatable("argument.entity.notfound.entity", Vec::new()));
    }
    Ok(found)
}

/// `CommandSourceStack.getEntityOrException` (CommandSourceStack.java:393-399),
/// whose `ERROR_NOT_ENTITY` (`:53`) is `permissions.requires.entity`.
fn source_entity<'a>(
    source: &CommandSource,
    world: &EntityReads<'a>,
) -> Result<CommandDamageEntityRef<'a>, NbtSummaryValue> {
    source_entity_state(source, world)
        .ok_or_else(|| translatable("permissions.requires.entity", Vec::new()))
}

fn integer_argument(parse: &ParseResult, name: &str) -> i32 {
    match parse.argument(name) {
        Some(ArgumentValue::Integer(value)) => *value,
        other => unreachable!("{name} is an integer argument, not {other:?}"),
    }
}

fn time_argument(parse: &ParseResult, name: &str) -> i32 {
    match parse.argument(name) {
        Some(ArgumentValue::Time(ticks)) => *ticks,
        other => unreachable!("{name} is a time argument, not {other:?}"),
    }
}

fn float_argument(parse: &ParseResult, name: &str) -> f32 {
    match parse.argument(name) {
        Some(ArgumentValue::Float(value)) => *value,
        other => unreachable!("{name} is a float argument, not {other:?}"),
    }
}

fn double_argument(parse: &ParseResult, name: &str) -> f64 {
    match parse.argument(name) {
        Some(ArgumentValue::Double(value)) => *value,
        other => unreachable!("{name} is a double argument, not {other:?}"),
    }
}

fn identifier_argument<'a>(parse: &'a ParseResult, name: &str) -> &'a str {
    match parse.argument(name) {
        Some(ArgumentValue::Identifier(id)) => id,
        other => unreachable!("{name} is a resource location argument, not {other:?}"),
    }
}

fn resource_argument<'a>(parse: &'a ParseResult, name: &str) -> &'a str {
    match parse.argument(name) {
        Some(ArgumentValue::Resource(id)) => id,
        other => unreachable!("{name} is a resource argument, not {other:?}"),
    }
}

fn coordinates_argument(parse: &ParseResult, name: &str) -> Coordinates {
    match parse.argument(name) {
        Some(ArgumentValue::Coordinates(coordinates)) => *coordinates,
        other => unreachable!("{name} is a vec3 argument, not {other:?}"),
    }
}

fn vec2_argument(parse: &ParseResult, name: &str) -> Coordinates {
    match parse.argument(name) {
        Some(ArgumentValue::Vec2(coordinates)) => *coordinates,
        other => unreachable!("{name} is a vec2 argument, not {other:?}"),
    }
}

/// `RotationArgument.getRotation` (RotationArgument.java:21-23), which hands
/// back the `Coordinates` unresolved the way `Vec3Argument.getCoordinates`
/// does.
fn rotation_argument(parse: &ParseResult, name: &str) -> Coordinates {
    match parse.argument(name) {
        Some(ArgumentValue::Rotation(coordinates)) => *coordinates,
        other => unreachable!("{name} is a rotation argument, not {other:?}"),
    }
}

/// `EntityAnchorArgument.getAnchor` (EntityAnchorArgument.java:30-32).
fn entity_anchor_argument(parse: &ParseResult, name: &str) -> EntityAnchor {
    match parse.argument(name) {
        Some(ArgumentValue::EntityAnchor(anchor)) => *anchor,
        other => unreachable!("{name} is an entity anchor argument, not {other:?}"),
    }
}

/// `GameModeArgument.getGameMode` (GameModeArgument.java:53-55).
fn game_mode_argument(parse: &ParseResult, name: &str) -> GameType {
    match parse.argument(name) {
        Some(ArgumentValue::GameMode(game_type)) => *game_type,
        other => unreachable!("{name} is a gamemode argument, not {other:?}"),
    }
}

/// `BanListEntry.getReasonMessage` (BanListEntry.java:70-73): the stored reason
/// as a literal, or the default translatable when there is none.
fn ban_reason_message(reason: Option<&bbb_protocol::JavaString>) -> NbtSummaryValue {
    match reason {
        Some(reason) => reason.to_nbt(),
        None => translatable("multiplayer.disconnect.banned.reason.default", Vec::new()),
    }
}

fn difficulty_entry(
    difficulty: Difficulty,
) -> &'static bbb_protocol::difficulties::DifficultyCatalogEntry {
    VANILLA_DIFFICULTIES
        .iter()
        .find(|entry| entry.difficulty == difficulty)
        .expect("every difficulty is in the generated catalog")
}

/// `Difficulty.getDisplayName` (Difficulty.java:33-35).
fn difficulty_display_name(difficulty: Difficulty) -> NbtSummaryValue {
    translatable(
        format!(
            "options.difficulty.{}",
            difficulty_entry(difficulty).serialized_name
        ),
        Vec::new(),
    )
}

fn view_of(player: CommandPlayer<'_>) -> CommandPlayerView {
    CommandPlayerView {
        name: player.name.into(),
        uuid: player.id,
    }
}

fn component_argument<'a>(parse: &'a ParseResult, name: &str) -> &'a NbtSummaryValue {
    match parse.argument(name) {
        Some(ArgumentValue::Component(value)) => value,
        other => unreachable!("{name} is a component argument, not {other:?}"),
    }
}

fn text_argument<'a>(parse: &'a ParseResult, name: &str) -> &'a bbb_protocol::JavaString {
    match parse.argument(name) {
        Some(ArgumentValue::Text(text)) => text,
        other => unreachable!("{name} is a string argument, not {other:?}"),
    }
}

/// `EntityArgument.getPlayers` (EntityArgument.java:89-96): an empty result is
/// the refusal rather than an empty run.
fn players_argument(
    parse: &ParseResult,
    name: &str,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
) -> Result<Vec<CommandPlayerTarget>, NbtSummaryValue> {
    world.with_entity_reads(|reads| {
        Ok(player_states_argument(parse, name, source, reads, at)?
            .into_iter()
            .map(|player| player.target())
            .collect())
    })
}

/// The same resolution keeping the players themselves, which is what a command
/// reading more of a target than its name needs -- `/gamemode` compares each
/// one's current game type before it counts the move.
/// The players named by argument `name`, or the source player when the node
/// names none, as targets plus the first one's display name for a
/// single-target report.
fn player_targets(
    parse: &ParseResult,
    name: Option<&str>,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
) -> Result<(Vec<CommandPlayerTarget>, Option<NbtSummaryValue>), NbtSummaryValue> {
    world.with_entity_reads(|reads| {
        let players = match name {
            Some(name) => player_states_argument(parse, name, source, reads, at)?,
            None => {
                let Some(player) = source_player_state(source, reads) else {
                    return Err(translatable("permissions.requires.player", Vec::new()));
                };
                vec![player]
            }
        };
        let first_name = players
            .first()
            .map(|&player| team_player_display_name_nbt(&view_of(player), reads.scoreboard));
        Ok((
            players.iter().map(|player| player.target()).collect(),
            first_name,
        ))
    })
}

fn player_states_argument<'a>(
    parse: &ParseResult,
    name: &str,
    source: &CommandSource,
    world: &EntityReads<'a>,
    at: &CommandOrigin,
) -> Result<Vec<CommandPlayer<'a>>, NbtSummaryValue> {
    let Some(ArgumentValue::Players(selector)) = parse.argument(name) else {
        unreachable!("{name} is an entity argument");
    };
    let players = select_player_states(selector, source, at, world)?;
    if players.is_empty() {
        return Err(translatable("argument.entity.notfound.player", Vec::new()));
    }
    Ok(players)
}

/// `GameProfileArgument.Result.getNames`
/// (GameProfileArgument.java:63-66 and :106-119): a selector resolves to the
/// players it matched, and a plain name resolves through the name cache.
fn profiles_argument(
    parse: &ParseResult,
    name: &str,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
) -> Result<Vec<NameAndId>, NbtSummaryValue> {
    let Some(ArgumentValue::Profiles(selector)) = parse.argument(name) else {
        unreachable!("{name} is a game profile argument");
    };
    match &selector.form {
        // `CachedUserNameToIdResolver.get` (CachedUserNameToIdResolver.java:102-104)
        // looks the name up lower-cased, and `safeAdd` (:56-61) stored it under
        // the same key, so the cache answers `/op BOB` with the entry it holds
        // for Bob — under the casing it was stored with.
        //
        // A name the cache misses falls through to `lookupGameProfile`
        // (:63-70), which on a server that does not authenticate answers
        // `createUnknownProfile` (:72-74): `resolveOfflineUsers` is
        // `!usesAuthentication()` (DedicatedServer.java:280) and this server is
        // offline-only, so every name resolves. It resolves through the
        // *lower-cased* string, because that is what `get` hands the lookup
        // (:103, :118), which is also the id the login path already derives
        // for that player.
        SelectorForm::Named(profile) => {
            let lowered = profile.to_lowercase_root();
            let resolved = world
                .known_profiles()
                .into_iter()
                .find(|entry| entry.name.to_lowercase_root() == lowered)
                .unwrap_or_else(|| NameAndId::offline(&lowered));
            Ok(vec![resolved])
        }
        SelectorForm::Kind(_) => {
            let players = select_players(selector, source, world, at)?;
            if players.is_empty() {
                return Err(translatable("argument.entity.notfound.player", Vec::new()));
            }
            Ok(players)
        }
    }
}

/// `EntitySelector.findPlayers` for the forms this server resolves, as the
/// identities the list commands report.
fn select_players(
    selector: &Selector,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
) -> Result<Vec<NameAndId>, NbtSummaryValue> {
    world.with_entity_reads(|reads| {
        Ok(select_player_states(selector, source, at, reads)?
            .into_iter()
            .map(|player| player.user())
            .collect())
    })
}

/// `PlayerList.getPlayerByName` (PlayerList.java:584-595): the online player
/// list is searched with `equalsIgnoreCase` (:589).
///
/// Session names are ASCII (`StringUtil.java:66-68`), so their bytes can be
/// compared directly with a query's units.
fn player_name_matches(player: &str, name: &bbb_protocol::JavaString) -> bool {
    player.len() == name.units().len()
        && player
            .bytes()
            .zip(name.units())
            .all(|(byte, unit)| *unit <= 0x7f && byte.eq_ignore_ascii_case(&(*unit as u8)))
}

fn source_player_state<'a>(
    source: &CommandSource,
    world: &EntityReads<'a>,
) -> Option<CommandPlayer<'a>> {
    if source.identity != CommandSourceIdentity::Player {
        return None;
    }
    world.players().find(|player| player.id == source.uuid)
}

pub(super) fn source_entity_state<'a>(
    source: &CommandSource,
    world: &EntityReads<'a>,
) -> Option<CommandDamageEntityRef<'a>> {
    match &source.identity {
        CommandSourceIdentity::Player => {
            source_player_state(source, world).map(CommandDamageEntityRef::Player)
        }
        CommandSourceIdentity::StoredEntity {
            entity_id,
            dimension,
            ..
        } => world
            .command_entity_by_id(dimension, *entity_id)
            .filter(|entity| match entity {
                CommandDamageEntityRef::Stored(stored) => stored.entity.uuid() == source.uuid,
                CommandDamageEntityRef::Player(_) => false,
            }),
        CommandSourceIdentity::Server | CommandSourceIdentity::Block { .. } => None,
    }
}

fn styled_translatable(
    key: &str,
    arguments: Vec<NbtSummaryValue>,
    style: &ComponentStyle,
) -> NbtSummaryValue {
    bbb_protocol::component_translatable_nbt(key, arguments, style)
        .expect("command feedback styles are representable")
}

pub(crate) use item::{container_slot, player_slot_accepts, ItemCommand, ItemSlot};
pub(super) use scoreboard::score_holder_names_for_suggest;
pub(crate) use selector::block_description_id_opt;
pub(crate) use text::{resolve as resolve_text, resolve_entity_text, resolve_loot_text};
