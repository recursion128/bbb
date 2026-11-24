//! The server command dispatcher.
//!
//! This is the sole command owner: the chat-command packet handlers and the
//! MCP `command.execute` administration surface both route here rather than
//! parsing command strings themselves. A command runs against the live
//! `ServerWorld` through [`CommandCx`]: it reads and writes the world directly,
//! so a multi-target command that fails part way keeps the targets it already
//! changed, as in vanilla. Every published root runs on one queue, which is
//! [`execution`].
//!
//! The shape of every command — which nodes exist, which arguments they take,
//! what those arguments parse and which `PermissionLevel` each node needs — is
//! the derived Vanilla 26.1 tree in `bbb_protocol::packets`. The one fact this
//! crate owns is [`catalog::IMPLEMENTED_COMMANDS`], the root literals this
//! server runs; everything else is read off the catalog, so a published tree, a
//! parse and a completion always describe the same dispatcher. They do not all
//! apply `canUse` to it: publishing and parsing do, completion does not, which
//! is vanilla's own split and is spelled out in [`suggest`].
//!
//! A command belongs on that list only when the world can carry it out.
//! Publishing one that reports vanilla's success and
//! changes nothing is worse than not publishing it at all, because the client
//! and the server then disagree about what happened; [`catalog`] names the two
//! commands this crate can already word but the world cannot yet act on.
//!
//! Vanilla owners: `Commands.performCommand` (`Commands.java:319-356`) for the
//! dispatch and failure path, and one registration class per command.

use bbb_protocol::{
    chat_formattings::ChatFormatting,
    component_appended_nbt, component_hover_uuid_nbt, component_literal_nbt,
    component_styled_literal_nbt, component_translatable_nbt, component_with_siblings_nbt,
    entity_types::{vanilla_entity_resource_id_for_type_id, VANILLA_ENTITY_TYPE_PLAYER_ID},
    packets::CommandSuggestion,
    ComponentClickEvent, ComponentHoverEvent, ComponentStyle, NbtSummaryValue, RichTextComponent,
};
use uuid::Uuid;

use bbb_server_types::save::Java26LevelKey;

use crate::world::scoreboard::{PlayerTeam, Scoreboard};

mod catalog;
mod dialog;
mod execution;
mod item_modifier;
mod item_predicate;
mod loot_predicate;
mod loot_table;
pub(crate) use loot_table::CommandLootTable;
mod parse;
mod properties;
mod run;
mod selector_advancements;
mod selector_options;
pub(crate) use bbb_server_types::slot_ranges::{player_slot, PlayerSlot};
mod suggest;
pub mod tree;
pub(crate) mod types;

pub(crate) use execution::{
    execute_entry, run_command_queue, EntryOutput, ExecutionContext, ExecutionSource,
    QueuedFunction, ResultAccumulator, ResultCallback,
};
pub(crate) use run::{accessor_data, avatar_profile_feedback, failure_feedback};
pub(crate) use run::{
    block_description_id_opt, block_predicate_matches, runtime_item_stack_display_name,
    CloneBlocksRequest, PlaceCommandKind, PlaceCommandRequest,
};
pub(crate) use run::{sprint_report, LocateKind, LocateRequest, LocateSearchResult};
pub use run::{CommandBroadcast, CommandFeedback, CommandFeedbackKind, CommandRun};
pub(crate) use run::{FetchProfileResult, FetchProfileTarget, RecipeCommandAction};
pub use types::{
    game_mode_change_moves_session, CommandAdvancementAction, CommandAttributeTarget,
    CommandBlockInput, CommandBlockNeighbourPass, CommandBlockPlacement, CommandBlockResultTarget,
    CommandBossBarMutation, CommandBossBarState, CommandCameraTarget, CommandClockChange,
    CommandDamageSourceEntity, CommandDataTarget, CommandDatapack, CommandDebugProfilerSample,
    CommandDebugProfilerView, CommandEntityDataState, CommandExperienceKind, CommandInventory,
    CommandLookAt, CommandOrigin, CommandPackDescription, CommandPlayer, CommandPlayerTarget,
    CommandScheduledCallback, CommandScheduledEvent, CommandSoundRecipient, CommandStoredEntity,
    CommandWaypointState, CommandWeatherKind, TeleportMove,
};
pub(crate) use types::{
    CommandFunctionBody, CommandFunctionDefinition, CommandFunctionEntry, CommandFunctionTemplate,
};

/// Permission a command requires.
///
/// Vanilla 26.1 replaced the numeric op levels with named checks
/// (`Commands.LEVEL_ALL`..`LEVEL_OWNERS`, `Commands.java:162-166`) over the
/// five `PermissionLevel` values. The ordering is what
/// `PermissionLevel.isEqualOrHigherThan` (PermissionLevel.java:26-28) needs, so
/// it is derived from the enum order rather than from a parallel numeric table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CommandPermission {
    /// `Commands.LEVEL_ALL`: `PermissionCheck.AlwaysPass`.
    All,
    /// `Commands.LEVEL_MODERATORS`: `Permissions.COMMANDS_MODERATOR`.
    Moderator,
    /// `Commands.LEVEL_GAMEMASTERS`: `Permissions.COMMANDS_GAMEMASTER`.
    GameMaster,
    /// `Commands.LEVEL_ADMINS`: `Permissions.COMMANDS_ADMIN`.
    Admin,
    /// `Commands.LEVEL_OWNERS`: `Permissions.COMMANDS_OWNER`.
    Owner,
}

/// `ClientboundEntityEventPacket` id for `PermissionLevel.ALL`
/// (`PlayerList.sendPlayerPermissionLevel`, PlayerList.java:559-572). The
/// remaining four follow it in ladder order, which is what the switch there
/// spells out one line at a time.
const PERMISSION_LEVEL_ENTITY_EVENT_BASE: u8 = 24;

impl CommandPermission {
    /// The ladder in `PermissionLevel.values()` order, which is also the order
    /// its ids and its entity events are assigned in.
    const LADDER: [Self; 5] = [
        Self::All,
        Self::Moderator,
        Self::GameMaster,
        Self::Admin,
        Self::Owner,
    ];

    /// `PermissionLevel.id()` (PermissionLevel.java:34-36).
    pub fn id(self) -> u8 {
        Self::LADDER
            .iter()
            .position(|level| *level == self)
            .expect("every level is on the ladder") as u8
    }

    /// The level `PermissionLevel.id()` names, which is how the derived command
    /// tree records what a node requires.
    pub fn from_id(id: u8) -> Option<Self> {
        Self::LADDER.get(usize::from(id)).copied()
    }

    /// The entity event `PlayerList.sendPlayerPermissionLevel` announces this
    /// level to its own player with.
    pub fn entity_event_id(self) -> i8 {
        i8::try_from(PERMISSION_LEVEL_ENTITY_EVENT_BASE + self.id())
            .expect("the five event ids are all below 128")
    }
}

/// The entity, server, or player behind a command source.
#[derive(Debug, Clone, PartialEq)]
pub enum CommandSourceIdentity {
    Player,
    StoredEntity {
        entity_id: i32,
        dimension: Java26LevelKey,
        display_name: RichTextComponent,
    },
    Block {
        display_name: RichTextComponent,
    },
    Server,
}

/// Who is running a command and what they are allowed to run.
#[derive(Debug, Clone, PartialEq)]
pub struct CommandSource {
    pub name: bbb_protocol::JavaString,
    pub uuid: Uuid,
    pub permission: CommandPermission,
    pub identity: CommandSourceIdentity,
}

impl CommandSource {
    #[must_use]
    pub fn player(
        name: impl Into<bbb_protocol::JavaString>,
        uuid: Uuid,
        permission: CommandPermission,
    ) -> Self {
        Self {
            name: name.into(),
            uuid,
            permission,
            identity: CommandSourceIdentity::Player,
        }
    }

    #[must_use]
    pub fn stored_entity(
        name: impl Into<bbb_protocol::JavaString>,
        uuid: Uuid,
        entity_id: i32,
        dimension: Java26LevelKey,
        display_name: RichTextComponent,
        permission: CommandPermission,
    ) -> Self {
        Self {
            name: name.into(),
            uuid,
            permission,
            identity: CommandSourceIdentity::StoredEntity {
                entity_id,
                dimension,
                display_name,
            },
        }
    }

    #[must_use]
    pub fn server(
        name: impl Into<bbb_protocol::JavaString>,
        permission: CommandPermission,
    ) -> Self {
        Self {
            name: name.into(),
            uuid: Uuid::nil(),
            permission,
            identity: CommandSourceIdentity::Server,
        }
    }

    pub fn block(display_name: RichTextComponent) -> Self {
        Self {
            name: bbb_server_datapack::language::component_plain_text(display_name.component())
                .into(),
            uuid: Uuid::nil(),
            permission: CommandPermission::GameMaster,
            identity: CommandSourceIdentity::Block { display_name },
        }
    }
}

/// One player as the dispatcher sees them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandPlayerView {
    pub name: bbb_protocol::JavaString,
    pub uuid: Uuid,
}

/// Wraps a locally built Component tree.
///
/// The builders below only assemble shapes the Component codec already
/// validates, so a rejection is a bug in this module rather than untrusted
/// input; failing loudly keeps a malformed Component from reaching the wire.
fn component(value: NbtSummaryValue) -> RichTextComponent {
    component_with_registry(
        value,
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
    )
}

fn component_with_registry(
    value: NbtSummaryValue,
    registry: &bbb_protocol::content_registry::ContentRegistry,
) -> RichTextComponent {
    RichTextComponent::try_from_component_nbt_with_registry(value, registry)
        .expect("command components are built from registered content")
}

/// `PlayerList.placeNewPlayer` (PlayerList.java:193-200): the arrival the
/// player list announces to everyone already on it.
///
/// The renamed form is the other half of that branch and cannot be reached
/// here: it needs the profile name to have changed since the last session,
/// which offline-only identity never does.
pub(crate) fn player_joined_announcement(
    player: &CommandPlayerView,
    scoreboard: &Scoreboard,
) -> RichTextComponent {
    presence_announcement("multiplayer.player.joined", player, scoreboard)
}

/// `ServerGamePacketListenerImpl.handleSetCommandBlock`
/// (ServerGamePacketListenerImpl.java:606-607) and `handleSetCommandMinecart`
/// (:665-666): the two `canUseGameMasterBlocks` gates are the only places in
/// that file that send this key, and each sends it to the player who asked for
/// it and to nobody else.
pub fn command_block_not_allowed() -> RichTextComponent {
    component(translatable("advMode.notAllowed", Vec::new()))
}

pub fn command_block_set_feedback(
    command: impl Into<bbb_protocol::JavaString>,
    enabled: bool,
) -> RichTextComponent {
    component(translatable(
        if enabled {
            "advMode.setCommand.success"
        } else {
            "advMode.setCommand.disabled"
        },
        vec![NbtSummaryValue::string(command)],
    ))
}

/// `ServerGamePacketListenerImpl.INVALID_COMMAND_SIGNATURE` (:237), the
/// refusal `handleMessageDecodeFailure` (:1552-1555) sends when the signatures
/// a command carried do not name the arguments the parse read.
pub fn invalid_command_signature() -> RichTextComponent {
    component(
        component_translatable_nbt(
            "chat.disabled.invalid_command_signature",
            Vec::new(),
            &chat_formatting_style(ChatFormatting::Red),
        )
        .expect("the refusal style is representable"),
    )
}

/// `ServerPlayer.sendBuildLimitMessage` (ServerPlayer.java:1791-1793).
pub fn build_limit_message(too_high: bool, limit: i32) -> RichTextComponent {
    component(
        component_translatable_nbt(
            if too_high {
                "build.tooHigh"
            } else {
                "build.tooLow"
            },
            vec![NbtSummaryValue::Int(limit)],
            &chat_formatting_style(ChatFormatting::Red),
        )
        .expect("the refusal style is representable"),
    )
}

/// `ServerPlayer.sendSpawnProtectionMessage` (ServerPlayer.java:1795-1797).
pub fn spawn_protection_message(x: i32, y: i32, z: i32) -> RichTextComponent {
    component(
        component_translatable_nbt(
            "build.spawn_protection",
            vec![NbtSummaryValue::string(format!("{x}, {y}, {z}"))],
            &chat_formatting_style(ChatFormatting::Red),
        )
        .expect("the refusal style is representable"),
    )
}

/// The reason `ServerCommonPacketListenerImpl.disconnect` (:141-151) puts in
/// `ClientboundDisconnectPacket`. Every vanilla disconnect message in this
/// server's reach is an unstyled, argument-free translatable.
pub fn disconnect_reason(key: &str) -> RichTextComponent {
    component(translatable(key, Vec::new()))
}

/// Which ban list refused a login, which decides both translation keys
/// `PlayerList.canPlayerLogin` uses (PlayerList.java:336-342 for the player ban
/// and :349-355 for the address ban).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BanKind {
    Player,
    Ip,
}

impl BanKind {
    fn reason_key(self) -> &'static str {
        match self {
            Self::Player => "multiplayer.disconnect.banned.reason",
            Self::Ip => "multiplayer.disconnect.banned_ip.reason",
        }
    }

    fn expiration_key(self) -> &'static str {
        match self {
            Self::Player => "multiplayer.disconnect.banned.expiration",
            Self::Ip => "multiplayer.disconnect.banned_ip.expiration",
        }
    }
}

/// The refusal `PlayerList.canPlayerLogin` builds for a banned login.
///
/// The reason is `BanListEntry.getReasonMessage` (BanListEntry.java:70-73): the
/// stored reason as a literal, or the default translatable when the entry
/// carries none. A ban with an expiry appends the expiration line as a sibling
/// of the reason component itself (`MutableComponent.append`), not as a second
/// message. `expires` arrives already formatted, because
/// `PlayerList.BAN_DATE_FORMAT` (PlayerList.java:113) is a document format and
/// the world crate owns no clock or locale.
pub fn ban_refusal(
    kind: BanKind,
    reason: Option<&bbb_protocol::JavaString>,
    expires: Option<&str>,
) -> RichTextComponent {
    let reason = reason.map_or_else(
        || translatable("multiplayer.disconnect.banned.reason.default", Vec::new()),
        bbb_protocol::JavaString::to_nbt,
    );
    let refusal = translatable(kind.reason_key(), vec![reason]);
    let siblings = expires
        .map(|expires| {
            vec![translatable(
                kind.expiration_key(),
                vec![component_literal_nbt(expires)],
            )]
        })
        .unwrap_or_default();
    component(
        component_with_siblings_nbt(refusal, siblings)
            .expect("a translatable refusal accepts siblings"),
    )
}

/// `ServerGamePacketListenerImpl.removePlayerFromWorld` (:1450-1459): the
/// departure, announced before the player leaves the list.
pub(crate) fn player_left_announcement(
    player: &CommandPlayerView,
    scoreboard: &Scoreboard,
) -> RichTextComponent {
    presence_announcement("multiplayer.player.left", player, scoreboard)
}

/// Both announcements are the same shape — the key over `Player.getDisplayName`
/// in yellow — and share the display name with command feedback rather than
/// spelling it a second time.
fn presence_announcement(
    key: &str,
    player: &CommandPlayerView,
    scoreboard: &Scoreboard,
) -> RichTextComponent {
    component(
        component_translatable_nbt(
            key,
            vec![team_player_display_name_nbt(player, scoreboard)],
            &chat_formatting_style(ChatFormatting::Yellow),
        )
        .expect("a yellow translatable over a display name is representable"),
    )
}

/// Vanilla `MutableComponent.withStyle(ChatFormatting)`, which resolves the
/// constant through `TextColor.fromLegacyFormat` (`TextColor.java:69-71`) and
/// keeps the name beside the value.
///
/// Command feedback is where the distinction is observable: a named colour
/// serializes as its name (`TextColor.serialize`, `TextColor.java:41-47`), so
/// vanilla's refusals carry `color:"red"` on the wire and only an anonymous
/// RGB colour would carry `color:"#FF5555"`. Every colour below therefore has
/// to travel as a `ChatFormatting`, never as a bare `0xRRGGBB` value.
fn chat_formatting_style(formatting: ChatFormatting) -> ComponentStyle {
    ComponentStyle::with_chat_formatting_color(formatting)
        .expect("command feedback only uses ChatFormatting colours, never format codes")
}

/// `Commands.performCommand` (Commands.java:319-356) up to the queue: the parse
/// is finished first, and only a chain that flattened opens an execution
/// context, which the caller then drains against the live world.
///
/// The string arrives without the leading slash, matching
/// `ServerboundChatCommandPacket.command()`. `Err` is the run a parse failure
/// answers with.
pub(crate) fn prepare_command(
    command: std::sync::Arc<[u16]>,
    source: &CommandSource,
    world: &crate::world::ServerWorld,
    origin: &CommandOrigin,
    callback: ResultCallback,
    chat_signer: Option<Uuid>,
) -> Result<ExecutionContext, CommandRun> {
    // `CommandSourceStack.entity` is the source's own until an `execute as`
    // replaces it (`CommandSourceStack.java:151-169`), and it is what an `EYES`
    // anchor measures from (EntityAnchorArgument.java:85-88).
    let origin = CommandOrigin {
        eye_height: run::source_eye_height(source, &world.entity_reads()),
        ..origin.clone()
    };
    // `requires()` removes a node from parsing altogether, so an unpermitted
    // command is indistinguishable from an unregistered one and the failure
    // never leaks which commands exist.
    let parsed = parse::parse(&command, source.permission, world);
    if parsed.failure(&command).is_some() || parsed.executable_node().is_none() {
        return Err(parse_failure_run(&command, &parsed));
    }
    let mut context = ExecutionContext::open(world);
    context.queue_initial_command_execution(
        command,
        parsed,
        ExecutionSource {
            source: source.clone(),
            origin,
            silent: false,
            trace: None,
            executing: None,
            callback,
            chat_signer,
        },
    );
    Ok(context)
}

/// The live world one command entry reads and writes.
pub(crate) struct CommandCx<'w> {
    pub(crate) world: &'w mut crate::world::ServerWorld,
    pub(crate) events: &'w mut Vec<crate::world::WorldEvent>,
    /// Where the running source stands.
    pub(crate) origin: CommandOrigin,
    /// The running entry's `ExecutionSource::chat_signer`.
    pub(crate) chat_signer: Option<Uuid>,
}

/// Whether a `.mcfunction` line compiles, which is
/// `CommandFunction.fromLines`'s dispatcher half
/// (CommandFunction.java:77-81 through `parseCommand`, `:96-107`).
///
/// A line that does not compile takes the whole function down with it: vanilla
/// wraps the `CommandSyntaxException` in an `IllegalArgumentException` (`:80`)
/// and `ServerFunctionLibrary` logs the failed future and leaves the id out of
/// the map (ServerFunctionLibrary.java:104-113). So this answers the question
/// the loader asks, not the one the runner does.
pub fn function_line_compiles(line: &[u16], world: &crate::world::ServerWorld) -> bool {
    let parsed = parse::parse(line, world.function_compilation_permission(), world);
    parsed.failure(line).is_none() && parsed.executable_node().is_some()
}

/// What `CommandDispatcher.getCompletionSuggestions`
/// (CommandDispatcher.java:287-324) answers for this dispatcher.
///
/// `Suggestions.EMPTY` is a range at zero with no entries (Suggestions.java:13),
/// which is what `merge`/`create` (:62-103) fall back to whenever nothing
/// matched — so an empty answer carries no offset at all.
///
/// The entries are the packet's own [`CommandSuggestion`] rather than a
/// parallel struct, because that is exactly what the answer is: `Suggestions`
/// reaches the client through `ClientboundCommandSuggestionsPacket`
/// (ClientboundCommandSuggestionsPacket.java:31-45), which copies the text and
/// `Suggestion.getTooltip` straight over. A second shape here would be a second
/// place a tooltip could be dropped on the way out.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CommandCompletions {
    pub start: i32,
    pub length: i32,
    pub suggestions: Vec<CommandSuggestion>,
}

/// The suggestions `handleCustomCommandSuggestions`
/// (ServerGamePacketListenerImpl.java:589-601) asks for, over the world the
/// caller projected.
///
/// The whole source travels rather than its permission alone because one
/// published suggestion provider resolves an entity argument of the command
/// being typed: `/tag <targets> remove <name>` offers the tags of `<targets>`
/// (TagCommand.java:39), and a `@s` in that slot is the source's own entity.
pub fn completion_suggestions_in_world(
    command: &str,
    source: &CommandSource,
    world: &crate::world::ServerWorld,
    origin: &CommandOrigin,
) -> CommandCompletions {
    suggest::completions(command, source, world, origin)
}

/// `SignableCommand.of` (SignableCommand.java:18-31): the arguments a parse
/// actually read whose type is a `SignedArgument`.
///
/// `MessageArgument` is the only `SignedArgument` vanilla registers, so the
/// answer is the name of every `minecraft:message` node the parse walked
/// through. A parse that never reached one — a different command, a `/say`
/// with nothing after it, or a command the source may not use — read no
/// signable argument, exactly as `visitArguments` reports for a node it never
/// entered.
///
/// The names are what the caller needs: the values only ever reach
/// `SignedMessageChain.Decoder.unsigned` (SignedMessageChain.java:93-101),
/// which returns an unsigned message for any body while `enforceSecureProfile`
/// is off — and it is off, because this server is offline-only.
///
/// `world` is what the command itself parses against, because this is the
/// same parse: `performSignedChatCommand` (ServerGamePacketListenerImpl.java:1536-1541)
/// hands `SignableCommand.of` the `parseCommand` result for the very string it
/// is about to run, and an argument that reads the world —
/// `minecraft:resource` against the `minecraft:damage_type` registry — decides
/// where that parse stops.
pub fn signable_argument_names(
    command: &[u16],
    permission: CommandPermission,
    world: &crate::world::ServerWorld,
) -> Vec<&'static str> {
    parse::parse(command, permission, world).signable_arguments()
}

/// `ComponentUtils.formatList` (ComponentUtils.java:116-138): no values give
/// the empty component, one value is that value verbatim (both branches at
/// :117-123), and more are joined by the `DEFAULT_SEPARATOR` the shorter
/// overload passes in — the gray `", "` literal of :19 — under an empty root.
fn format_list(entries: Vec<NbtSummaryValue>) -> NbtSummaryValue {
    let mut entries = entries.into_iter();
    let Some(first) = entries.next() else {
        return component_appended_nbt(&ComponentStyle::default(), Vec::new())
            .expect("the empty component carries no style");
    };
    let rest = entries.collect::<Vec<_>>();
    if rest.is_empty() {
        return first;
    }
    let separator = styled_literal(", ", &chat_formatting_style(ChatFormatting::Gray));
    let mut children = vec![first];
    for entry in rest {
        children.push(separator.clone());
        children.push(entry);
    }
    component_appended_nbt(&ComponentStyle::default(), children)
        .expect("the list root carries no style")
}

/// `ComponentUtils.formatList(Collection<String>)` (ComponentUtils.java:84-86)
/// through `formatAndSortList` (`:88-100`), which is the only list helper that
/// sorts: the names go out in natural order, each as a green literal.
fn format_and_sort_list<T: Ord + Into<bbb_protocol::JavaString>>(
    mut names: Vec<T>,
) -> NbtSummaryValue {
    names.sort();
    format_list(
        names
            .into_iter()
            .map(|name| styled_literal(name, &chat_formatting_style(ChatFormatting::Green)))
            .collect(),
    )
}

/// `Player.getDisplayName` (Player.java:1676-1686) for a player on no team.
pub(crate) fn player_display_name_nbt(player: &CommandPlayerView) -> NbtSummaryValue {
    styled_literal(&player.name, &player_decoration(player))
}

/// `Player.getDisplayName` (Player.java:1676-1686) with the player's team.
pub(crate) fn team_player_display_name_nbt(
    player: &CommandPlayerView,
    scoreboard: &Scoreboard,
) -> NbtSummaryValue {
    match scoreboard.players_team(&player.name) {
        None => player_display_name_nbt(player),
        Some(team) => format_name_for_team(
            team,
            component_literal_nbt(&player.name),
            player_decoration(player),
        ),
    }
}

/// `Player.decorateDisplayNameComponent` (Player.java:1681-1686).
fn player_decoration(player: &CommandPlayerView) -> ComponentStyle {
    ComponentStyle {
        click_event: Some(ComponentClickEvent::SuggestCommand {
            command: "/tell "
                .encode_utf16()
                .chain(player.name.units().iter().copied())
                .chain([u16::from(b' ')])
                .collect::<Vec<_>>()
                .into(),
        }),
        // `Entity.createHoverEvent` (Entity.java:3358-3360).
        hover_event: Some(ComponentHoverEvent::ShowEntity {
            id: vanilla_entity_resource_id_for_type_id(VANILLA_ENTITY_TYPE_PLAYER_ID)
                .expect("the player entity type is in the generated catalog"),
            uuid: component_hover_uuid_nbt(player.uuid),
            name: None,
            name_component: Some(component_literal_nbt(&player.name)),
        }),
        insertion: Some(player.name.clone()),
        ..ComponentStyle::default()
    }
}

/// `PlayerTeam.formatNameForTeam` (PlayerTeam.java:116-128) with `decoration`
/// applied over the result, as `getDisplayName` does.
fn format_name_for_team(
    team: &PlayerTeam,
    name: NbtSummaryValue,
    decoration: ComponentStyle,
) -> NbtSummaryValue {
    let mut style = decoration;
    if let Some(color) = ComponentStyle::with_chat_formatting_color(team.color) {
        style.color = color.color;
        style.color_name = color.color_name;
    }
    let empty = || component_literal_nbt("");
    component_appended_nbt(
        &style,
        vec![
            team.player_prefix.clone().unwrap_or_else(empty),
            name,
            team.player_suffix.clone().unwrap_or_else(empty),
        ],
    )
    .expect("team names are components")
}

/// `CommandSourceStack.getDisplayName`: a player source reports the same
/// decorated display name the player list shows.
fn source_display_name_nbt(source: &CommandSource, scoreboard: &Scoreboard) -> NbtSummaryValue {
    match &source.identity {
        CommandSourceIdentity::Player => team_player_display_name_nbt(
            &CommandPlayerView {
                name: source.name.clone(),
                uuid: source.uuid,
            },
            scoreboard,
        ),
        CommandSourceIdentity::StoredEntity { display_name, .. }
        | CommandSourceIdentity::Block { display_name } => display_name.component().clone(),
        CommandSourceIdentity::Server => component_literal_nbt(&source.name),
    }
}

/// `Commands.finishParsing` (Commands.java:366-388): the raw error goes out
/// first, then -- only for an exception that carries a cursor -- a separate
/// cursor-context message that quotes the input around the failure position
/// and ends with the `command.context.here` marker.
fn parse_failure(command: &[u16], failure: parse::ParseFailure) -> CommandRun {
    // Both lines leave through `sendFailure`, so both carry its red root.
    let trace_errors = vec![(0, failure.message.clone())];
    let mut feedback = vec![run::failure_feedback(failure.message)];
    if let Some(cursor) = failure.cursor {
        feedback.push(run::failure_feedback(cursor_context(command, cursor)));
    }
    CommandRun {
        feedback,
        trace_errors,
        ..CommandRun::default()
    }
}

/// `Commands.finishParsing` over a whole parse: the error
/// `getParseException` surfaced (Commands.java:469-479), or the
/// `dispatcherUnknownCommand` a chain that would not flatten ends in (`:365`).
fn parse_failure_run(command: &[u16], parsed: &parse::ParseResult) -> CommandRun {
    let failure = parsed
        .failure(command)
        .unwrap_or_else(|| parse::ParseFailure {
            message: translatable("command.unknown.command", Vec::new()),
            cursor: Some(parsed.cursor),
        });
    parse_failure(command, failure)
}

fn cursor_context(command: &[u16], cursor: usize) -> NbtSummaryValue {
    let cursor = cursor.min(command.len());
    let mut children = Vec::new();
    if cursor > 10 {
        // `CommonComponents.ELLIPSIS` (CommonComponents.java:29).
        children.push(component_literal_nbt("..."));
    }
    let quoted_start = cursor.saturating_sub(10);
    children.push(component_literal_nbt(&command[quoted_start..cursor]));
    if cursor < command.len() {
        children.push(styled_literal(
            &command[cursor..],
            &ComponentStyle {
                underlined: Some(true),
                ..chat_formatting_style(ChatFormatting::Red)
            },
        ));
    }
    children.push(
        component_translatable_nbt(
            "command.context.here",
            Vec::new(),
            &ComponentStyle {
                italic: Some(true),
                ..chat_formatting_style(ChatFormatting::Red)
            },
        )
        .expect("the here marker style is representable"),
    );
    component_appended_nbt(
        &ComponentStyle {
            click_event: Some(ComponentClickEvent::SuggestCommand {
                command: std::iter::once(u16::from(b'/'))
                    .chain(command.iter().copied())
                    .collect::<Vec<_>>()
                    .into(),
            }),
            ..chat_formatting_style(ChatFormatting::Gray)
        },
        children,
    )
    .expect("the context style is representable")
}

/// `CommandSourceStack.broadcastToAdmins` (CommandSourceStack.java:479-493):
/// the copy of a `sendSuccess(.., true)` line the other operators and the
/// console receive, which is `chat.type.admin` over the source's display name
/// and the message, styled gray and italic (`:480`).
///
/// Only the component is built here. Who receives it is the world's question:
/// every online player that is not the source and is on the operator list,
/// when `SEND_COMMAND_FEEDBACK` is set (`:482-488`), and the console when
/// `LOG_ADMIN_COMMANDS` is set and the source is not the server itself
/// (`:490-492`).
pub(crate) fn admin_broadcast_copy(
    source: &CommandSource,
    message: &RichTextComponent,
    scoreboard: &Scoreboard,
    registry: &bbb_protocol::content_registry::ContentRegistry,
) -> RichTextComponent {
    component_with_registry(
        component_translatable_nbt(
            "chat.type.admin",
            vec![
                source_display_name_nbt(source, scoreboard),
                message.component().clone(),
            ],
            &ComponentStyle {
                italic: Some(true),
                ..chat_formatting_style(ChatFormatting::Gray)
            },
        )
        .expect("the admin copy style is representable"),
        registry,
    )
}

/// `ReloadCommand.reloadPacks` (ReloadCommand.java:18-24): what a reload that
/// could not be built reports, after the command already answered.
pub fn reload_failure() -> RichTextComponent {
    component(translatable("commands.reload.failure", Vec::new()))
}

pub(crate) fn translatable(
    key: impl Into<bbb_protocol::JavaString>,
    arguments: Vec<NbtSummaryValue>,
) -> NbtSummaryValue {
    component_translatable_nbt(key, arguments, &ComponentStyle::default())
        .expect("an unstyled translatable is always representable")
}

fn styled_literal(
    text: impl Into<bbb_protocol::JavaString>,
    style: &ComponentStyle,
) -> NbtSummaryValue {
    component_styled_literal_nbt(text, style).expect("command styles are representable")
}

#[cfg(test)]
mod tests;

pub(crate) use item_modifier::CommandItemModifier;
pub(crate) use run::{container_slot, player_slot_accepts, ItemCommand, ItemSlot};
pub(crate) use run::{resolve_entity_text, resolve_loot_text, resolve_text};
