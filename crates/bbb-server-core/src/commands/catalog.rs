//! The command surface this server owns, expressed over the derived tree.
//!
//! `VANILLA_COMMAND_TREE` (`bbb_protocol::packets`) is the shape half of every
//! command: node kinds, argument parsers, argument properties, redirects and
//! `PermissionLevel` requirements all come from the probed official tree rather
//! than from a table written here. The single fact this module owns is which of
//! the ninety root commands this server actually runs, because that is a
//! property of bbb and not of vanilla.
//!
//! Sibling order is the generated table's Brigadier registration order; the
//! packet builder preserves it while filtering and numbering usable nodes.

use bbb_protocol::game_rules::vanilla_game_rule;
use bbb_protocol::packets::{
    vanilla_command_node, CommandNodeType, VanillaCommandNode, VANILLA_COMMAND_TREE_ROOT,
};

use bbb_server_datapack::startup::DEFAULT_ENABLED_FEATURES_26_1;

/// Every root literal this server implements, in the derived table's order.
///
/// `CommandPermission::Owner` sees every node, so the highest level is also the
/// whole implemented surface.
#[cfg(test)]
pub(super) fn implemented_roots() -> impl Iterator<Item = u16> {
    usable_children(VANILLA_COMMAND_TREE_ROOT, CommandPermission::Owner)
}

use super::CommandPermission;

/// The root literals of the derived tree this server runs.
///
/// Every published executable leaf has an executor in [`super::run`]. A root
/// may publish only the branches not named by [`UNREGISTERED_SUBTREES`].
///
/// `/effect` is published: sessions carry `ActiveMobEffects` and
/// `ActiveMobEffect::give` builds the instance `giveEffect` does
/// (`EffectCommands.java:166`).
///
/// `/perf` and `/jfr` are refused one step earlier: the
/// blocker is not a store or a clock but a recorder this process does not run.
/// `/perf` gates both arms on `MinecraftServer.isRecordingMetrics`
/// (MinecraftServer.java:1949-1951), arms the recorder through
/// `startRecordingMetrics` (MinecraftServer.java:1953-1960), and reports
/// through the callbacks that zip a metrics directory into
/// `MetricsPersister.PROFILING_RESULTS_DIR` (PerfCommand.java:67-93) once
/// `finishRecordingMetrics` (MinecraftServer.java:1966-1968) has drained it;
/// `/jfr start` answers whichever way `JvmProfiler.INSTANCE.start` did
/// (JfrCommand.java:37-45). The
/// `/debug` TimeProfiler this server does run is a different recorder writing a
/// different report, so publishing either of these over it would name a report
/// that was never written.
///
/// The aliases are their own root literals in the derived tree — `tell` and `w`
/// redirect to `msg` (MsgCommand.java:29-30), `tm` redirects to `teammsg`
/// (TeamMsgCommand.java:57), `tp` redirects to `teleport`
/// (TeleportCommand.java:143) and `xp` redirects to `experience`
/// (ExperienceCommand.java:122) — so they are listed beside the command they
/// redirect to; a redirect whose target is missing would publish a dangling
/// node, which [`redirect_target`] refuses.
pub(super) const IMPLEMENTED_COMMANDS: &[&str] = &[
    // AdvancementCommands.java:32-217.
    "advancement",
    // AttributeCommand.java:42-223.
    "attribute",
    // BossBarCommands.java:30-333.
    "bossbar",
    // BanPlayerCommands.java:21-34 and BanIpCommands.java:23-36.
    "ban",
    "ban-ip",
    // BanListCommands.java:14-25.
    "banlist",
    // ClearInventoryCommands.java:27-54.
    "clear",
    // CloneCommands.java:46-58.
    "clone",
    // DamageCommand.java:21-101.
    "damage",
    // DataCommands.java:55-163.
    "data",
    // DataPackCommand.java:97-163, whole: `list` reads the last-load catalog
    // (`:231-274`), `enable` and `disable` hand `reloadPacks` the selection
    // they built (`:213-229`), and `create` writes a pack directory under
    // `LevelResource.DATAPACK_DIR` (`:165-211`).
    "datapack",
    // DebugCommand.java:53-69. `start` / `stop` are
    // `MinecraftServer` TimeProfiler (`MinecraftServer.java:2022-2037`).
    "debug",
    // DefaultGameModeCommands.java:12-18.
    "defaultgamemode",
    // DeOpCommands.java:18-28.
    "deop",
    // DialogCommand.java:17-41.
    "dialog",
    // DifficultyCommand.java:18-30.
    "difficulty",
    // EffectCommands.java:33-137.
    "effect",
    // EnchantCommand.java:39-111.
    "enchant",
    // ExecuteCommand.java:144-272.
    "execute",
    // ExperienceCommand.java:25-123, whose `xp` alias is the redirect the
    // last line of the same method registers (`:122`).
    "experience",
    // FillCommand.java:37-94.
    "fill",
    // FillBiomeCommand.java:41-79.
    "fillbiome",
    // ForceLoadCommand.java:34-78.
    "forceload",
    // FunctionCommand.java:63-95.
    "function",
    // GameModeCommand.java:21-34.
    "gamemode",
    // GameRuleCommand.java:15-26.
    "gamerule",
    // GiveCommand.java:23-46.
    "give",
    // HelpCommand.java:17-51.
    "help",
    // ItemCommands.java:61-305.
    "item",
    // KickCommand.java:20-33.
    "kick",
    // KillCommand.java:13-19.
    "kill",
    // ListPlayersCommand.java:15-19.
    "list",
    // LocateCommand.java:56-85.
    "locate",
    // LootCommand.java:56-257.
    "loot",
    // EmoteCommands.java:11-20.
    "me",
    // MsgCommand.java:17-31.
    "msg",
    // OpCommand.java:18-35.
    "op",
    // PardonCommand.java:18-28 and PardonIpCommand.java:18-28.
    "pardon",
    "pardon-ip",
    // ParticleCommand.java:24-140.
    "particle",
    // PlaceCommand.java:62-84; live writers are exposed beside feature.
    "place",
    // PlaySoundCommand.java:33-54, whose ten source literals are one body
    // registered over `SoundSource.values()` (`:49-51`, `:56-145`).
    "playsound",
    // RandomCommand.java:37-111.
    "random",
    // ReloadCommand.java:40-52.
    "reload",
    // RecipeCommand.java:21-70.
    "recipe",
    // FetchProfileCommand.java:31-57, minus `name` and `id` in
    // [`UNREGISTERED_SUBTREES`].
    "fetchprofile",
    // ReturnCommand.java:20-30. All three children are the execution engine
    // reaching its own frame rather than the world: `<value>` and `fail` set
    // the running frame's return value and discard it (`:32-39`, `:62-70`),
    // and `run` is the one `CustomModifierExecutor` this server publishes
    // (`:41-59`).
    "return",
    // RideCommand.java:35-50.
    "ride",
    // RotateCommand.java:16-63.
    "rotate",
    // SaveAllCommand.java:14-21, SaveOffCommand.java:12-23 and
    // SaveOnCommand.java:12-23.
    "save-all",
    "save-off",
    "save-on",
    // SayCommand.java:11-24.
    "say",
    // ScheduleCommand.java:38-79.
    "schedule",
    // ScoreboardCommand.java:68-369. Both halves are published: `objectives`
    // add/list/modify/remove/setdisplay and `players`
    // add/display/enable/get/list/operation/remove/reset/set.
    "scoreboard",
    // SeedCommand.java:10-19.
    "seed",
    // SetBlockCommand.java:23-97.
    "setblock",
    // SetPlayerIdleTimeoutCommand.java:10-19.
    "setidletimeout",
    // SetWorldSpawnCommand.java:16-54.
    "setworldspawn",
    // SetSpawnCommand.java:23-97.
    "spawnpoint",
    // SpectateCommand.java:24-37.
    "spectate",
    // SpreadPlayersCommand.java:46-100.
    "spreadplayers",
    // StopCommand.java:9-15.
    "stop",
    // StopSoundCommand.java:20-45, whose eleven source literals are one body
    // registered over `SoundSource.values()` (`:32-43`) beside the `*`
    // any-source literal (`:26-31`).
    "stopsound",
    // StopwatchCommand.java:29-66, plus the `if`/`unless` `stopwatch` arm
    // (ExecuteCommand.java:735-748) that reads the same map.
    "stopwatch",
    // SummonCommand.java:35-68.
    "summon",
    // SwingCommand.java:21-32.
    "swing",
    // TagCommand.java:22-45.
    "tag",
    // TeamCommand.java:61-231.
    "team",
    // TeamMsgCommand.java:28-58.
    "teammsg",
    // TeleportCommand.java:37-144, whose `tp` alias is the redirect the last
    // line of the same method registers (`:143`).
    "teleport",
    // TickCommand.java:20-55.
    "tick",
    // TimeCommand.java:46-62, whole: `query gametime` is
    // `ServerLevel.getGameTime()` (`:131-135`, `LevelAccessor.java:41-43`) and
    // every other arm runs on `ServerClockManager` (`:137-201`).
    "time",
    "tell",
    // TellRawCommand.java:12-29.
    "tellraw",
    // TitleCommand.java:26-101.
    "title",
    "tm",
    "tp",
    // TransferCommand.java:22-52.
    "transfer",
    // TriggerCommand.java:31-67.
    "trigger",
    // VersionCommand.java:17-26, whose `checkPermissions` argument is
    // `commandSelection != CommandSelection.INTEGRATED` (Commands.java:236-237)
    // and so is true on a dedicated server -- the `LEVEL_GAMEMASTERS` the
    // derived tree recorded rather than the `LEVEL_ALL` the other arm gives.
    "version",
    "w",
    // WaypointCommand.java:33-85.
    "waypoint",
    // WeatherCommand.java:15-42. Both arms of all three literals; a bare
    // literal draws its duration off the source level's random
    // (`:44-46`).
    "weather",
    // WhitelistCommand.java:32-69.
    "whitelist",
    // WorldBorderCommand.java:49-123.
    "worldborder",
    "xp",
];

/// The node at `index`.
///
/// Every index this module hands out came from the derived table itself, so an
/// index it cannot resolve is a bug here rather than untrusted input.
pub(super) fn node(index: u16) -> &'static VanillaCommandNode {
    vanilla_command_node(index).expect("every index comes from the derived tree")
}

/// The redirect of `index` as an index into the derived table.
///
/// `fillUsableCommands` (Commands.java:419-437) rebuilds a redirect against the
/// node it already converted, so a redirect whose target this server does not
/// implement could not be published at all.
pub(super) fn redirect_target(index: u16) -> Option<u16> {
    let redirect = node(index).redirect?;
    // `/execute run` redirects at the dispatcher root (`ExecuteCommand.java:151`),
    // which is not a command literal; every other published redirect names a
    // root this server implements.
    debug_assert!(
        redirect == VANILLA_COMMAND_TREE_ROOT
            || IMPLEMENTED_COMMANDS.contains(&node(redirect).name),
        "a published redirect points at a command this server does not implement"
    );
    Some(redirect)
}

/// The `PermissionLevel` a source needs before `parseNodes` will even look at
/// this node.
///
/// Brigadier gates a branch on the node the predicate sits on, and
/// `fillUsableCommands` recurses only through children that passed `canUse`, so
/// what a node really costs is the highest requirement on the path down to it.
pub(super) fn effective_permission(index: u16) -> CommandPermission {
    let mut level = node(index).permission;
    let mut current = node(index).parent;
    while let Some(parent) = current {
        level = level.max(node(parent).permission);
        current = node(parent).parent;
    }
    CommandPermission::from_id(level).expect("the derived tree carries PermissionLevel ids")
}

/// Whether `permission` may see this node at all.
///
/// The two halves are not the same rule. Not implementing a command is not
/// registering it, so it is absent from this dispatcher altogether — from the
/// published tree, from parsing and from completion alike. The permission half
/// is `canUse`, which `parseNodes` and `fillUsableCommands` both apply and
/// `getCompletionSuggestions` does not; [`implemented_children`] is what a
/// caller that must not apply it reads.
pub(super) fn is_usable(index: u16, permission: CommandPermission) -> bool {
    implemented(index) && permission >= effective_permission(index)
}

/// `ResourceArgument.resource(context, Registries.ATTRIBUTE)`
/// (`AttributeCommand.java:49`).
pub(super) const ATTRIBUTE_REGISTRY: &str = "minecraft:attribute";

/// `ResourceArgument.resource(context, Registries.ENCHANTMENT)`
/// (`EnchantCommand.java:46`).
pub(super) const ENCHANTMENT_REGISTRY: &str = "minecraft:enchantment";

/// `ResourceArgument.resource(context, Registries.MOB_EFFECT)`
/// (`EffectCommands.java:44`, `:54`).
pub(super) const MOB_EFFECT_REGISTRY: &str = "minecraft:mob_effect";

/// `ResourceKeyArgument.key(Registries.ADVANCEMENT)` (`AdvancementCommands.java:43`).
pub(super) const ADVANCEMENT_REGISTRY: &str = "minecraft:advancement";

/// `ResourceKeyArgument.key(Registries.CONFIGURED_FEATURE)` (`PlaceCommand.java:69`).
pub(super) const CONFIGURED_FEATURE_REGISTRY: &str = "minecraft:worldgen/configured_feature";

/// `ResourceKeyArgument.key(Registries.RECIPE)` (`RecipeCommand.java:30`, `:52`).
pub(super) const RECIPE_REGISTRY: &str = "minecraft:recipe";

/// `ResourceArgument.resource(context, Registries.ENTITY_TYPE)`
/// (`SummonCommand.java:40`).
pub(super) const ENTITY_TYPE_REGISTRY: &str = "minecraft:entity_type";

/// `ResourceArgument.resource(context, Registries.WORLD_CLOCK)`
/// (`TimeCommand.java:56`).
pub(super) const WORLD_CLOCK_REGISTRY: &str = "minecraft:world_clock";

/// `ResourceArgument.resource(context, Registries.TIMELINE)`
/// (`TimeCommand.java:103`).
pub(super) const TIMELINE_REGISTRY: &str = "minecraft:timeline";

/// The subtrees of an implemented root that this server does not register.
///
/// This is the second cut, and it is not the same rule as
/// [`IMPLEMENTED_COMMANDS`]: that one refuses a whole command, this one refuses
/// a branch of one.
///
const UNREGISTERED_SUBTREES: &[&[&str]] = &[
    // FetchProfileCommand.java:31-57. The two profile-service lookups are not
    // available on this offline-only server.
    &["fetchprofile", "name"],
    &["fetchprofile", "id"],
];

/// Whether the node hangs off a root literal this server implements, is not
/// inside one of the [`UNREGISTERED_SUBTREES`], and — if it is part of a
/// `/gamerule` rule — whether the rule is one this server's feature set has at
/// all.
fn implemented(index: u16) -> bool {
    let root = root_literal(index).name;
    if !IMPLEMENTED_COMMANDS.contains(&root) {
        return false;
    }
    if is_in_unregistered_subtree(root, index) {
        return false;
    }
    match game_rule_literal(index) {
        Some(literal) => game_rule_is_enabled(node(literal).name),
        None => true,
    }
}

/// Whether one of [`UNREGISTERED_SUBTREES`] names this node or an ancestor of
/// it, which is what makes the cut reach the whole branch rather than its root.
///
/// The root literal decides whether the path is worth building at all, because
/// this runs once per child of every node the dispatcher publishes or parses.
fn is_in_unregistered_subtree(root: &str, index: u16) -> bool {
    if !UNREGISTERED_SUBTREES
        .iter()
        .any(|subtree| subtree.first() == Some(&root))
    {
        return false;
    }
    let path = canonical_path(index);
    UNREGISTERED_SUBTREES
        .iter()
        .any(|subtree| path.len() >= subtree.len() && &path[..subtree.len()] == *subtree)
}

/// The `/gamerule` rule literal `index` is or hangs off, if any.
///
/// `GameRuleCommand.register` (GameRuleCommand.java:15-26) builds the rule
/// children of the `gamerule` literal and nothing else does, so a rule literal
/// is exactly a child of that root and the `value` argument under it is its
/// only descendant.
fn game_rule_literal(index: u16) -> Option<u16> {
    let mut current = index;
    loop {
        let parent = node(current).parent?;
        if parent == VANILLA_COMMAND_TREE_ROOT {
            return None;
        }
        if node(parent).parent == Some(VANILLA_COMMAND_TREE_ROOT) && node(parent).name == "gamerule"
        {
            return Some(current);
        }
        current = parent;
    }
}

/// Whether the rule that literal names survives this server's feature set.
///
/// Vanilla does not publish a node that refuses: `register` visits
/// `new GameRules(context.enabledFeatures())` (GameRuleCommand.java:17), which
/// keeps only the rules whose `requiredFeatures` are enabled
/// (GameRules.java:112-114), so a rule behind an experiment has no node in the
/// tree, no completion and no parse. The derived table was probed from a tree
/// built with every feature on, which is why the cut is made here instead.
///
/// Both spellings are looked up because vanilla registers each rule twice, once
/// under `gameRule.id()` and once under the qualified identifier
/// (GameRuleCommand.java:21-23); the catalog is keyed by the qualified one.
fn game_rule_is_enabled(name: &str) -> bool {
    let qualified = if name.contains(':') {
        name.to_string()
    } else {
        format!("minecraft:{name}")
    };
    match vanilla_game_rule(&qualified) {
        Some(entry) => entry.is_enabled(DEFAULT_ENABLED_FEATURES_26_1),
        // A literal under `gamerule` that names no rule is not a rule literal;
        // 26.1 registers none, and refusing to publish one would be this
        // module inventing a cut rather than reproducing vanilla's.
        None => true,
    }
}

/// The root literal `index` hangs off, or the node itself when it is one.
fn root_literal(index: u16) -> &'static VanillaCommandNode {
    let mut current = index;
    while let Some(parent) = node(current).parent {
        if parent == VANILLA_COMMAND_TREE_ROOT {
            break;
        }
        current = parent;
    }
    node(current)
}

/// The children of `index` that `permission` may use, in the table's order.
pub(super) fn usable_children(
    index: u16,
    permission: CommandPermission,
) -> impl Iterator<Item = u16> {
    node(index)
        .children()
        .map(|child| child as u16)
        .filter(move |child| is_usable(*child, permission))
}

/// The children of `index` this server registered at all, in the table's order.
///
/// This is the whole child list of a node of the server's own dispatcher, which
/// is what completion walks: `handleCustomCommandSuggestions` asks
/// `this.server.getCommands().getDispatcher()`
/// (ServerGamePacketListenerImpl.java:596-597) — the unfiltered dispatcher, not
/// the per-source tree `sendCommands` published — and
/// `CommandDispatcher.getCompletionSuggestions`
/// (com/mojang/brigadier/CommandDispatcher.java:299-311) iterates
/// `parent.getChildren()` with no `canUse` check anywhere on the way.
pub(super) fn implemented_children(index: u16) -> impl Iterator<Item = u16> {
    node(index)
        .children()
        .map(|child| child as u16)
        .filter(|child| implemented(*child))
}

/// The path from the root down to `index`, one name per node.
///
/// This is the path the executors match on. It is taken from the derived tree's
/// parent links rather than from the nodes a parse walked through, so an alias
/// resolves to the command it redirects to: `/tell` reaches `msg`'s argument
/// nodes and therefore reports the path `msg targets message`.
pub(super) fn canonical_path(index: u16) -> Vec<&'static str> {
    let mut names = Vec::new();
    let mut current = Some(index);
    while let Some(node_index) = current {
        if node_index == VANILLA_COMMAND_TREE_ROOT {
            break;
        }
        names.push(node(node_index).name);
        current = node(node_index).parent;
    }
    names.reverse();
    names
}

/// Whether the node's command is a `CustomCommandExecutor` rather than a
/// Brigadier `Command`.
///
/// `BuildContexts` runs such a node itself (BuildContexts.java:102-107) instead
/// of scheduling an `ExecuteCommand` for it, which is the whole difference:
/// `incrementCost` lives in `ExecuteCommand.execute` (ExecuteCommand.java:28),
/// so a custom executor spends no command quota. Vanilla registers three of
/// them — `FunctionCommand` (FunctionCommand.java:230-232), `DebugCommand` and
/// `ReturnCommand` (ReturnCommand.java:32-39, `:62-70`).
pub(super) fn is_custom_executor(index: u16) -> bool {
    let path = canonical_path(index);
    matches!(
        path.as_slice(),
        ["debug", "function", "name"]
            | ["function", "name"]
            | ["function", "name", "arguments"]
            | ["return", "value"]
            | ["return", "fail"]
    ) || path.starts_with(&["function", "name", "with"])
}

/// Whether the node is a literal, which decides both the parse rule
/// (`CommandNode.getRelevantNodes` matches literals by exact word) and the
/// suggestion rule (`LiteralCommandNode.listSuggestions`,
/// LiteralCommandNode.java:67-69).
pub(super) fn is_literal(index: u16) -> bool {
    node(index).kind == CommandNodeType::Literal
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The index of one implemented root literal, by name.
    fn implemented_root(name: &str) -> u16 {
        implemented_roots()
            .find(|index| node(*index).name == name)
            .unwrap_or_else(|| panic!("{name} is implemented"))
    }

    /// The index of one root literal of the derived tree, implemented or not.
    fn root_by_name(name: &str) -> u16 {
        node(VANILLA_COMMAND_TREE_ROOT)
            .children()
            .map(|child| child as u16)
            .find(|index| node(*index).name == name)
            .unwrap_or_else(|| panic!("{name} is a root literal of the derived tree"))
    }

    #[test]
    fn every_implemented_command_names_a_root_literal_of_the_derived_tree() {
        for name in IMPLEMENTED_COMMANDS {
            assert!(
                implemented_roots().any(|index| node(index).name == *name),
                "{name} is not a root literal of the derived command tree"
            );
        }
        assert_eq!(implemented_roots().count(), IMPLEMENTED_COMMANDS.len());
    }

    #[test]
    fn every_implemented_alias_redirects_to_an_implemented_command() {
        for index in implemented_roots() {
            let Some(redirect) = redirect_target(index) else {
                continue;
            };
            assert!(
                IMPLEMENTED_COMMANDS.contains(&node(redirect).name),
                "{} redirects to {}, which is not implemented",
                node(index).name,
                node(redirect).name
            );
        }
    }

    #[test]
    fn a_nodes_effective_permission_is_the_highest_one_on_its_path() {
        // `/difficulty easy` carries no requirement of its own
        // (DifficultyCommand.java:22) and is only reachable through the
        // `difficulty` literal, which requires LEVEL_GAMEMASTERS (:25).
        let difficulty = implemented_root("difficulty");
        let easy = node(difficulty)
            .children()
            .find(|child| node(*child as u16).name == "easy")
            .expect("the derived tree carries /difficulty easy") as u16;
        assert_eq!(node(easy).permission, 0);
        assert_eq!(
            effective_permission(easy),
            CommandPermission::GameMaster,
            "the literal above it is what carries the requirement"
        );
    }

    #[test]
    fn help_uses_the_derived_registration_order() {
        let difficulty = implemented_root("difficulty");
        let published = node(difficulty)
            .children()
            .map(|child| node(child as u16).name)
            .collect::<Vec<_>>();
        assert_eq!(
            published,
            ["peaceful", "easy", "normal", "hard"],
            "the derived table preserves CommandNode.getChildren registration order"
        );
        assert!(IMPLEMENTED_COMMANDS.contains(&"help"));
    }

    #[test]
    fn getsmartusage_chooses_brackets_from_the_parent_executable() {
        // `getSmartUsage` picks `[`/`]` over `(`/`)` from the parent's own
        // command, not from its children
        // (com/mojang/brigadier/CommandDispatcher.java:233-235, :264-278), and
        // that bit did survive the probe as `executable`. `/difficulty` has an
        // `executes` of its own (DifficultyCommand.java:25) so vanilla prints
        // `[peaceful|easy|normal|hard]`; `/whitelist` only has `requires`
        // (WhitelistCommand.java:34-35) so vanilla prints
        // `(on|off|list|add|remove|reload)`.
        assert!(
            node(implemented_root("difficulty")).executable,
            "an executable parent is what makes vanilla print square brackets"
        );
        assert!(
            !node(implemented_root("whitelist")).executable,
            "a parent with no command of its own is what makes vanilla print parentheses"
        );
    }

    #[test]
    fn effect_stays_out_while_a_player_is_not_a_living_entity_here() {
        // Sessions carry `ActiveMobEffects`; `/effect` files instances with
        // `ActiveMobEffect::give` (`EffectCommands.java:166`). `new` still
        // refuses infinite duration and fixes particles at true.
        assert!(IMPLEMENTED_COMMANDS.contains(&"effect"));
        assert!(
            crate::entity::living::mob_effects::ActiveMobEffect::new("minecraft:speed", -1, 0)
                .is_none(),
            "the runtime now spells an infinite effect"
        );
        assert!(
            crate::entity::living::mob_effects::ActiveMobEffect::new("minecraft:speed", 20, 0)
                .is_some_and(|effect| effect.show_particles()),
            "the runtime now spells a hidden-particle effect"
        );
        assert!(
            crate::entity::living::mob_effects::ActiveMobEffect::give(
                "minecraft:speed",
                -1,
                0,
                true
            )
            .is_some(),
            "`give` is what `/effect give .. infinite` builds (`:103`, `:113`)"
        );
        assert!(
            crate::entity::living::mob_effects::ActiveMobEffect::give(
                "minecraft:speed",
                20,
                0,
                false
            )
            .is_some_and(|effect| !effect.show_particles()),
            "`give` is what `<hideParticles>` (`:85-95`) builds"
        );
    }

    #[test]
    fn stopwatch_stays_out_with_the_execute_arm_that_reads_one() {
        // `ExecuteCommand.checkStopwatch` (ExecuteCommand.java:849-860) reads
        // the same `getStopwatches()` map and throws the same
        // `StopwatchCommand.ERROR_DOES_NOT_EXIST` (StopwatchCommand.java:22-24)
        // the root exports, so the arm and the root move together.
        assert!(IMPLEMENTED_COMMANDS.contains(&"stopwatch"));
        for arm in [
            ["execute", "if", "stopwatch"],
            ["execute", "unless", "stopwatch"],
        ] {
            assert!(
                !UNREGISTERED_SUBTREES.contains(&arm.as_slice()),
                "{arm:?} has to move with /stopwatch"
            );
        }
        assert_eq!(
            effective_permission(root_by_name("stopwatch")),
            CommandPermission::GameMaster,
            "`Commands.LEVEL_GAMEMASTERS` (StopwatchCommand.java:32)"
        );
    }

    #[test]
    fn datapack_publishes_every_arm_register_builds() {
        assert!(IMPLEMENTED_COMMANDS.contains(&"datapack"));
        for arm in [
            ["datapack", "enable"].as_slice(),
            ["datapack", "disable"].as_slice(),
            ["datapack", "list"].as_slice(),
            ["datapack", "create"].as_slice(),
        ] {
            assert!(
                !UNREGISTERED_SUBTREES.contains(&arm),
                "{arm:?} is published"
            );
        }
        assert_eq!(
            effective_permission(root_by_name("datapack")),
            CommandPermission::GameMaster,
            "`Commands.LEVEL_GAMEMASTERS` (DataPackCommand.java:100)"
        );
        let create = node(root_by_name("datapack"))
            .children()
            .map(|child| child as u16)
            .find(|index| node(*index).name == "create")
            .expect("the derived tree hangs create off datapack");
        assert_eq!(
            effective_permission(create),
            CommandPermission::Owner,
            "`Commands.LEVEL_OWNERS` (DataPackCommand.java:151)"
        );
    }

    #[test]
    fn summon_publishes_the_nbt_arm() {
        assert!(IMPLEMENTED_COMMANDS.contains(&"summon"));
        assert!(!UNREGISTERED_SUBTREES.contains(&["summon", "entity", "pos", "nbt"].as_slice()));
        assert_eq!(
            effective_permission(root_by_name("summon")),
            CommandPermission::GameMaster,
            "`Commands.LEVEL_GAMEMASTERS` (SummonCommand.java:38)"
        );
    }

    #[test]
    fn loot_publishes_the_sources_and_destinations() {
        assert!(IMPLEMENTED_COMMANDS.contains(&"loot"));
        for arm in [
            ["loot", "replace"].as_slice(),
            ["loot", "insert"].as_slice(),
            ["loot", "spawn", "targetPos", "kill"].as_slice(),
            ["loot", "spawn", "targetPos", "mine"].as_slice(),
            ["loot", "give", "players", "kill"].as_slice(),
            ["loot", "give", "players", "mine"].as_slice(),
        ] {
            assert!(
                !UNREGISTERED_SUBTREES.contains(&arm),
                "{arm:?} has a live /loot writer"
            );
        }
        assert_eq!(
            effective_permission(root_by_name("loot")),
            CommandPermission::GameMaster,
            "`Commands.LEVEL_GAMEMASTERS` (LootCommand.java:59)"
        );
    }

    #[test]
    fn place_publishes_the_arms_with_live_writers() {
        assert!(IMPLEMENTED_COMMANDS.contains(&"place"));
        for arm in [
            ["place", "jigsaw"].as_slice(),
            ["place", "structure"].as_slice(),
            ["place", "template"].as_slice(),
        ] {
            assert!(
                !UNREGISTERED_SUBTREES.contains(&arm),
                "{arm:?} follows its live writer availability"
            );
        }
        assert_eq!(
            effective_permission(root_by_name("place")),
            CommandPermission::GameMaster,
            "`Commands.LEVEL_GAMEMASTERS` (PlaceCommand.java:65)"
        );
    }

    #[test]
    fn locate_publishes_every_search_arm() {
        assert!(IMPLEMENTED_COMMANDS.contains(&"locate"));
        // LocateCommand.java:60-84 registers all three searches.
        for arm in [
            ["locate", "structure"].as_slice(),
            ["locate", "biome"].as_slice(),
            ["locate", "poi"].as_slice(),
        ] {
            assert!(
                !UNREGISTERED_SUBTREES.contains(&arm),
                "{arm:?} follows its live search owner"
            );
        }
        assert_eq!(
            effective_permission(root_by_name("locate")),
            CommandPermission::GameMaster,
            "`Commands.LEVEL_GAMEMASTERS` (LocateCommand.java:58)"
        );
    }

    #[test]
    fn the_two_profiler_roots_stay_out_at_the_level_the_derived_tree_records() {
        // Both require LEVEL_OWNERS (PerfCommand.java:38, JfrCommand.java:31),
        // which is what the probe recorded for them. `/debug` is the one
        // profiler root this server publishes and it drives a different
        // recorder, so its presence is not a reason to publish these.
        for name in ["perf", "jfr"] {
            assert!(!IMPLEMENTED_COMMANDS.contains(&name));
            assert_eq!(
                effective_permission(root_by_name(name)),
                CommandPermission::Owner
            );
        }
        assert!(IMPLEMENTED_COMMANDS.contains(&"debug"));
    }

    #[test]
    fn setidletimeout_writes_a_world_field_rather_than_an_admin_setting() {
        // `setPlayerIdleTimeout` lands in `server.properties` on a dedicated
        // server (DedicatedServer.java:611-613) and is read back as
        // `serversettings/player_idle_timeout` (IncomingRpcMethods.java:224-227),
        // so the admin surface would be the obvious home for it. It is not
        // one: `scripts/tests/test_admin_settings_alignment.py` lists
        // `player_idle_timeout` among the properties this server does not
        // model as a setting, and that list is what says a key is deliberately
        // absent rather than forgotten. The value lives on the world instead,
        // where the tick that reads it is.
        assert!(IMPLEMENTED_COMMANDS.contains(&"setidletimeout"));
        assert!(
            !crate::admin::ADMIN_SETTING_KEYS.contains(&"player_idle_timeout"),
            "the alignment guard owns this key as unmodelled"
        );
        assert_eq!(
            effective_permission(root_by_name("setidletimeout")),
            CommandPermission::Admin,
            "`Commands.LEVEL_ADMINS` (SetPlayerIdleTimeoutCommand.java:13)"
        );
    }

    #[test]
    fn the_execute_subtree_this_server_publishes_is_run_in_source_modifiers_and_as_at() {
        let execute = implemented_root("execute");
        let mut names = implemented_children(execute)
            .map(|child| node(child).name)
            .collect::<Vec<_>>();
        names.sort_unstable();
        assert_eq!(
            names,
            vec![
                "align",
                "anchored",
                "as",
                "at",
                "facing",
                "if",
                "in",
                "on",
                "positioned",
                "rotated",
                "run",
                "store",
                "summon",
                "unless"
            ]
        );
        let run = implemented_children(execute)
            .find(|child| node(*child).name == "run")
            .expect("`/execute run` is published");
        assert_eq!(redirect_target(run), Some(VANILLA_COMMAND_TREE_ROOT));
        let in_node = implemented_children(execute)
            .find(|child| node(*child).name == "in")
            .expect("`/execute in` is published");
        let dimension = implemented_children(in_node)
            .next()
            .expect("`/execute in <dimension>` is published");
        assert_eq!(
            canonical_path(dimension),
            vec!["execute", "in", "dimension"]
        );
        assert_eq!(node(dimension).parser, Some("minecraft:dimension"));
        assert_eq!(redirect_target(dimension), Some(execute));
        let positioned = implemented_children(execute)
            .find(|child| node(*child).name == "positioned")
            .expect("`/execute positioned` is published");
        let pos = implemented_children(positioned)
            .find(|child| node(*child).name == "pos")
            .expect("`/execute positioned <pos>` is published");
        assert_eq!(canonical_path(pos), vec!["execute", "positioned", "pos"]);
        assert_eq!(node(pos).parser, Some("minecraft:vec3"));
        assert_eq!(redirect_target(pos), Some(execute));
        let positioned_as = implemented_children(positioned)
            .find(|child| node(*child).name == "as")
            .expect("`/execute positioned as` is published");
        let positioned_as_targets = implemented_children(positioned_as)
            .next()
            .expect("`/execute positioned as <targets>` is published");
        assert_eq!(
            canonical_path(positioned_as_targets),
            vec!["execute", "positioned", "as", "targets"]
        );
        assert_eq!(node(positioned_as_targets).parser, Some("minecraft:entity"));
        assert_eq!(redirect_target(positioned_as_targets), Some(execute));
        let positioned_over = implemented_children(positioned)
            .find(|child| node(*child).name == "over")
            .expect("`/execute positioned over` is published");
        let heightmap = implemented_children(positioned_over)
            .next()
            .expect("`/execute positioned over <heightmap>` is published");
        assert_eq!(
            canonical_path(heightmap),
            vec!["execute", "positioned", "over", "heightmap"]
        );
        assert_eq!(node(heightmap).parser, Some("minecraft:heightmap"));
        assert_eq!(redirect_target(heightmap), Some(execute));
        let rotated = implemented_children(execute)
            .find(|child| node(*child).name == "rotated")
            .expect("`/execute rotated` is published");
        let rot = implemented_children(rotated)
            .find(|child| node(*child).name == "rot")
            .expect("`/execute rotated <rot>` is published");
        assert_eq!(canonical_path(rot), vec!["execute", "rotated", "rot"]);
        assert_eq!(node(rot).parser, Some("minecraft:rotation"));
        assert_eq!(redirect_target(rot), Some(execute));
        let rotated_as = implemented_children(rotated)
            .find(|child| node(*child).name == "as")
            .expect("`/execute rotated as` is published");
        let rotated_as_targets = implemented_children(rotated_as)
            .next()
            .expect("`/execute rotated as <targets>` is published");
        assert_eq!(
            canonical_path(rotated_as_targets),
            vec!["execute", "rotated", "as", "targets"]
        );
        assert_eq!(node(rotated_as_targets).parser, Some("minecraft:entity"));
        assert_eq!(redirect_target(rotated_as_targets), Some(execute));
        let facing = implemented_children(execute)
            .find(|child| node(*child).name == "facing")
            .expect("`/execute facing` is published");
        let facing_pos = implemented_children(facing)
            .find(|child| node(*child).name == "pos")
            .expect("`/execute facing <pos>` is published");
        assert_eq!(canonical_path(facing_pos), vec!["execute", "facing", "pos"]);
        assert_eq!(node(facing_pos).parser, Some("minecraft:vec3"));
        assert_eq!(redirect_target(facing_pos), Some(execute));
        let facing_entity = implemented_children(facing)
            .find(|child| node(*child).name == "entity")
            .expect("`/execute facing entity` is published");
        let facing_targets = implemented_children(facing_entity)
            .next()
            .expect("`/execute facing entity <targets>` is published");
        assert_eq!(
            canonical_path(facing_targets),
            vec!["execute", "facing", "entity", "targets"]
        );
        let facing_anchor = implemented_children(facing_targets)
            .next()
            .expect("`/execute facing entity <targets> <anchor>` is published");
        assert_eq!(
            canonical_path(facing_anchor),
            vec!["execute", "facing", "entity", "targets", "anchor"]
        );
        assert_eq!(node(facing_anchor).parser, Some("minecraft:entity_anchor"));
        assert_eq!(redirect_target(facing_anchor), Some(execute));
        let as_node = implemented_children(execute)
            .find(|child| node(*child).name == "as")
            .expect("`/execute as` is published");
        let as_targets = implemented_children(as_node)
            .next()
            .expect("`/execute as <targets>` is published");
        assert_eq!(canonical_path(as_targets), vec!["execute", "as", "targets"]);
        assert_eq!(node(as_targets).parser, Some("minecraft:entity"));
        assert_eq!(redirect_target(as_targets), Some(execute));
        let at_node = implemented_children(execute)
            .find(|child| node(*child).name == "at")
            .expect("`/execute at` is published");
        let at_targets = implemented_children(at_node)
            .next()
            .expect("`/execute at <targets>` is published");
        assert_eq!(canonical_path(at_targets), vec!["execute", "at", "targets"]);
        assert_eq!(node(at_targets).parser, Some("minecraft:entity"));
        assert_eq!(redirect_target(at_targets), Some(execute));
        let align = implemented_children(execute)
            .find(|child| node(*child).name == "align")
            .expect("`/execute align` is published");
        let axes = implemented_children(align)
            .next()
            .expect("`/execute align <axes>` is published");
        assert_eq!(canonical_path(axes), vec!["execute", "align", "axes"]);
        assert_eq!(node(axes).parser, Some("minecraft:swizzle"));
        assert_eq!(redirect_target(axes), Some(execute));
        let anchored = implemented_children(execute)
            .find(|child| node(*child).name == "anchored")
            .expect("`/execute anchored` is published");
        let anchor = implemented_children(anchored)
            .next()
            .expect("`/execute anchored <anchor>` is published");
        assert_eq!(
            canonical_path(anchor),
            vec!["execute", "anchored", "anchor"]
        );
        assert_eq!(node(anchor).parser, Some("minecraft:entity_anchor"));
        assert_eq!(redirect_target(anchor), Some(execute));
        for conditional in ["if", "unless"] {
            let arm = implemented_children(execute)
                .find(|child| node(*child).name == conditional)
                .expect("`/execute if` and `/execute unless` are published");
            let mut arms: Vec<_> = implemented_children(arm)
                .map(|child| node(child).name)
                .collect();
            arms.sort_unstable();
            assert_eq!(
                arms,
                vec![
                    "biome",
                    "block",
                    "blocks",
                    "data",
                    "dimension",
                    "entity",
                    "function",
                    "items",
                    "loaded",
                    "predicate",
                    "score",
                    "stopwatch"
                ],
                "{conditional}"
            );
            let pos = implemented_children(arm)
                .find(|child| node(*child).name == "block")
                .and_then(|block| implemented_children(block).next())
                .expect("`if block <pos>` is published");
            assert_eq!(node(pos).parser, Some("minecraft:block_pos"));
            let predicate = implemented_children(pos)
                .next()
                .expect("`if block <pos> <block>` is published");
            assert_eq!(node(predicate).parser, Some("minecraft:block_predicate"));
            assert_eq!(redirect_target(predicate), Some(execute));
            assert!(node(predicate).executable);
            let loaded = implemented_children(arm)
                .find(|child| node(*child).name == "loaded")
                .and_then(|loaded| implemented_children(loaded).next())
                .expect("`if loaded <pos>` is published");
            assert_eq!(node(loaded).parser, Some("minecraft:block_pos"));
            assert_eq!(redirect_target(loaded), Some(execute));
            assert!(node(loaded).executable);
            let biome_pos = implemented_children(arm)
                .find(|child| node(*child).name == "biome")
                .and_then(|biome| implemented_children(biome).next())
                .expect("`if biome <pos>` is published");
            assert_eq!(node(biome_pos).parser, Some("minecraft:block_pos"));
            let biome = implemented_children(biome_pos)
                .next()
                .expect("`if biome <pos> <biome>` is published");
            assert_eq!(node(biome).parser, Some("minecraft:resource_or_tag"));
            assert_eq!(redirect_target(biome), Some(execute));
            assert!(node(biome).executable);
            let function_name = implemented_children(arm)
                .find(|child| node(*child).name == "function")
                .and_then(|function| implemented_children(function).next())
                .expect("`if function <name>` is published");
            assert_eq!(node(function_name).parser, Some("minecraft:function"));
            assert_eq!(redirect_target(function_name), Some(execute));
            assert!(!node(function_name).executable);
            let stopwatch_id = implemented_children(arm)
                .find(|child| node(*child).name == "stopwatch")
                .and_then(|stopwatch| implemented_children(stopwatch).next())
                .expect("`if stopwatch <id>` is published");
            assert_eq!(
                node(stopwatch_id).parser,
                Some("minecraft:resource_location")
            );
            let range = implemented_children(stopwatch_id)
                .next()
                .expect("`if stopwatch <id> <range>` is published");
            assert_eq!(node(range).parser, Some("minecraft:float_range"));
            assert_eq!(redirect_target(range), Some(execute));
            assert!(node(range).executable);
            let score_target = implemented_children(arm)
                .find(|child| node(*child).name == "score")
                .and_then(|score| implemented_children(score).next())
                .expect("`if score <target>` is published");
            assert_eq!(node(score_target).parser, Some("minecraft:score_holder"));
            let target_objective = implemented_children(score_target)
                .next()
                .expect("`if score <target> <targetObjective>` is published");
            assert_eq!(node(target_objective).parser, Some("minecraft:objective"));
            let mut tests: Vec<_> = implemented_children(target_objective)
                .map(|child| node(child).name)
                .collect();
            tests.sort_unstable();
            assert_eq!(tests, vec!["<", "<=", "=", ">", ">=", "matches"]);
            for comparison in ["=", "<", "<=", ">", ">="] {
                let source = implemented_children(target_objective)
                    .find(|child| node(*child).name == comparison)
                    .and_then(|literal| implemented_children(literal).next())
                    .expect("every comparison publishes its source holder");
                assert_eq!(node(source).parser, Some("minecraft:score_holder"));
                let source_objective = implemented_children(source)
                    .next()
                    .expect("every comparison publishes its source objective");
                assert_eq!(node(source_objective).parser, Some("minecraft:objective"));
                assert_eq!(redirect_target(source_objective), Some(execute));
                assert!(node(source_objective).executable);
            }
            let score_range = implemented_children(target_objective)
                .find(|child| node(*child).name == "matches")
                .and_then(|matches| implemented_children(matches).next())
                .expect("`if score <target> <targetObjective> matches <range>` is published");
            assert_eq!(node(score_range).parser, Some("minecraft:int_range"));
            assert_eq!(redirect_target(score_range), Some(execute));
            assert!(node(score_range).executable);
            let data = implemented_children(arm)
                .find(|child| node(*child).name == "data")
                .expect("`if data` is published");
            let mut providers: Vec<_> = implemented_children(data)
                .map(|child| node(child).name)
                .collect();
            providers.sort_unstable();
            assert_eq!(
                providers,
                vec!["block", "entity", "storage"],
                "{conditional}"
            );
            for (provider, argument, parser) in [
                ("block", "sourcePos", "minecraft:block_pos"),
                ("entity", "source", "minecraft:entity"),
                ("storage", "source", "minecraft:resource_location"),
            ] {
                let accessor = implemented_children(data)
                    .find(|child| node(*child).name == provider)
                    .and_then(|child| implemented_children(child).next())
                    .expect("every published provider publishes its accessor argument");
                assert_eq!(node(accessor).name, argument);
                assert_eq!(node(accessor).parser, Some(parser));
                let path = implemented_children(accessor)
                    .next()
                    .expect("every published provider publishes its path");
                assert_eq!(node(path).name, "path");
                assert_eq!(node(path).parser, Some("minecraft:nbt_path"));
                assert_eq!(redirect_target(path), Some(execute));
                assert!(node(path).executable);
            }
            // `addIfBlocksConditional` hangs the two modes off the third
            // corner and both of them execute (`ExecuteCommand.java:619-630`).
            let mut corner = implemented_children(arm)
                .find(|child| node(*child).name == "blocks")
                .expect("`if blocks` is published");
            for name in ["start", "end", "destination"] {
                corner = implemented_children(corner)
                    .next()
                    .expect("every region corner is published");
                assert_eq!(node(corner).name, name, "{conditional}");
                assert_eq!(node(corner).parser, Some("minecraft:block_pos"));
                assert!(!node(corner).executable);
            }
            let mut modes: Vec<_> = implemented_children(corner)
                .map(|child| node(child).name)
                .collect();
            modes.sort_unstable();
            assert_eq!(modes, vec!["all", "masked"], "{conditional}");
            for mode in implemented_children(corner) {
                assert_eq!(redirect_target(mode), Some(execute));
                assert!(node(mode).executable);
            }
        }
        assert!(!node(run).executable);
        assert!(!node(dimension).executable);
    }

    #[test]
    fn the_function_subtree_publishes_explicit_and_data_arguments() {
        // `/function` publishes `<name>`, `<arguments>` and all three `with`
        // accessors (FunctionCommand.java:63-95).
        let function = implemented_root("function");
        let name = implemented_children(function)
            .next()
            .expect("`/function <name>` is published");
        assert_eq!(canonical_path(name), vec!["function", "name"]);
        assert!(node(name).executable);
        let arguments = implemented_children(name)
            .next()
            .expect("`/function <name> <arguments>` is published");
        assert_eq!(
            canonical_path(arguments),
            vec!["function", "name", "arguments"]
        );
        assert!(node(arguments).executable);
        assert_eq!(node(arguments).parser, Some("minecraft:nbt_compound_tag"));
        assert_eq!(implemented_children(name).count(), 2);
        let published = node(name)
            .children()
            .map(|child| node(child as u16).name)
            .collect::<Vec<_>>();
        assert_eq!(published, vec!["arguments", "with"]);
        let with = node(name)
            .children()
            .find(|child| node(*child as u16).name == "with")
            .expect("the derived tree carries with") as u16;
        assert!(implemented(with));
        let mut pending = vec![with];
        while let Some(parent) = pending.pop() {
            for child in implemented_children(parent) {
                if node(child).executable {
                    assert!(is_custom_executor(child), "{:?}", canonical_path(child));
                }
                pending.push(child);
            }
        }
        assert!(IMPLEMENTED_COMMANDS.contains(&"data"));
    }

    #[test]
    fn the_store_subtree_publishes_every_arm_wrap_stores_builds() {
        // `wrapStores` (`ExecuteCommand.java:274-412`) hangs five arms off each
        // of `result` and `success`. Every published node redirects back at
        // `/execute` and none of them executes, because a store is a modifier.
        let execute = implemented_root("execute");
        let store = implemented_children(execute)
            .find(|child| node(*child).name == "store")
            .expect("`/execute store` is published");
        for kind in ["result", "success"] {
            let arm = implemented_children(store)
                .find(|child| node(*child).name == kind)
                .expect("both store literals are published");
            assert_eq!(
                implemented_children(arm)
                    .map(|child| node(child).name)
                    .collect::<Vec<_>>(),
                vec!["score", "bossbar", "entity", "block", "storage"]
            );
            let score = implemented_children(arm)
                .find(|child| node(*child).name == "score")
                .expect("the score arm is published");
            let targets = implemented_children(score)
                .next()
                .expect("`score <targets>` is published");
            assert_eq!(node(targets).parser, Some("minecraft:score_holder"));
            let objective = implemented_children(targets)
                .next()
                .expect("`score <targets> <objective>` is published");
            assert_eq!(node(objective).parser, Some("minecraft:objective"));
            assert_eq!(redirect_target(objective), Some(execute));
            assert!(!node(objective).executable);
            let bossbar = implemented_children(arm)
                .find(|child| node(*child).name == "bossbar")
                .expect("the bossbar arm is published");
            let id = implemented_children(bossbar)
                .next()
                .expect("`bossbar <id>` is published");
            assert_eq!(node(id).parser, Some("minecraft:resource_location"));
            assert_eq!(
                implemented_children(id)
                    .map(|child| node(child).name)
                    .collect::<Vec<_>>(),
                vec!["value", "max"]
            );
            for into in implemented_children(id) {
                assert_eq!(redirect_target(into), Some(execute));
                assert!(!node(into).executable);
            }
            // The three `DataCommands.TARGET_PROVIDERS` (`:306-409`) each wrap
            // one accessor argument, then the shared `path` and the six tag
            // literals, each of which carries the `scale` that redirects.
            for (provider, argument, parser) in [
                ("entity", "target", "minecraft:entity"),
                ("block", "targetPos", "minecraft:block_pos"),
                ("storage", "target", "minecraft:resource_location"),
            ] {
                let arm = implemented_children(arm)
                    .find(|child| node(*child).name == provider)
                    .expect("every target provider is published");
                let target = implemented_children(arm)
                    .next()
                    .expect("the accessor argument is published");
                assert_eq!(node(target).name, argument);
                assert_eq!(node(target).parser, Some(parser));
                let path = implemented_children(target)
                    .next()
                    .expect("the path argument is published");
                assert_eq!(node(path).parser, Some("minecraft:nbt_path"));
                assert_eq!(
                    implemented_children(path)
                        .map(|child| node(child).name)
                        .collect::<Vec<_>>(),
                    vec!["int", "float", "short", "long", "double", "byte"]
                );
                for tag in implemented_children(path) {
                    let scale = implemented_children(tag)
                        .next()
                        .expect("every tag literal carries a scale");
                    assert_eq!(node(scale).name, "scale");
                    assert_eq!(node(scale).parser, Some("brigadier:double"));
                    assert_eq!(redirect_target(scale), Some(execute));
                    assert!(!node(scale).executable);
                }
            }
        }
    }

    #[test]
    fn an_alias_reports_the_canonical_path_of_the_command_it_redirects_to() {
        let tell = implemented_root("tell");
        let msg = redirect_target(tell).expect("tell redirects");
        assert_eq!(canonical_path(msg), vec!["msg"]);
    }
}
