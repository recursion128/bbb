//! The second, operator-privileged session the replay runs.
//!
//! The main replay is a player with no permissions, which is what makes its
//! refusals mean something and what leaves every command's *effect* and
//! feedback uncompared. This module is the other half: one more session, under
//! its own name, that the server already considers an operator, running a
//! fixed list of commands and recording what each one answered with.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use bbb_client_world::{CommandNodeKindState, CommandNodeState, CommandTreeState};
use bbb_protocol::{
    canonical_component_codec_value, canonical_snbt_string,
    nbt_summary::clone_nbt_with_last_wins_compounds,
    packets::{self, ClientIntent},
    CanonicalComponentCodecValue, NbtSummaryEntry, NbtSummaryValue,
};
use tokio::time::timeout;

use crate::types::{
    CommandFeedbackObservation, CommandMessageObservation, CommandTreeNodeObservation,
    DamageEventObservation, OperatorCommandTreeReport, OperatorDamageReport, OperatorSessionReport,
    PlayerHealthObservation, SoundObservation, OPERATOR_DAMAGE_COMMAND,
    OPERATOR_DAMAGE_INVULNERABLE_COMMAND, OPERATOR_PROBE_COMMANDS, OPERATOR_PROBE_FENCE_COMMAND,
    OPERATOR_PROBE_FENCE_TRANSLATION_KEY, OPERATOR_PROBE_USERNAME,
};
use bbb_client_assets::KnownPacksManager;
use bbb_client_net::{
    ClientIdentity, ConnectionOptions, DecodedClientboundPacket, PlayPacketBundleAssembler,
    RawConnection,
};
use std::sync::Arc;

use super::{sound_observation, ProbeContext};

/// How long the session waits to become a joined player holding a command
/// tree.
///
/// It is the same journey the main replay already made on this server, so
/// reaching this bound means the login stopped rather than that it was slow.
const OPERATOR_JOIN_TIMEOUT: Duration = Duration::from_secs(30);

/// How long one command may take to be answered.
///
/// What is waited for is the fence's refusal, not the command's own message,
/// and the fence is answered on the same server tick the command ran on — so
/// this only ever expires on a server that stopped answering.
const OPERATOR_COMMAND_TIMEOUT: Duration = Duration::from_secs(15);

/// Wait past `CLIENT_LOADED_TIMEOUT_TIME` (`ServerGamePacketListenerImpl.java:235`) so invulnerability clears deterministically.
const CLIENT_LOADED_TIMEOUT_TICKS: i64 = 60;

/// How deep the command-tree projection follows a node's children.
///
/// The published tree is a tree — `fillUsableCommands` (Commands.java:419-437)
/// builds a fresh node per child and hangs it off exactly one parent — so this
/// is not reached by any tree either server can legitimately send. It bounds
/// the walk over a *decoded* tree, whose child indices are whatever arrived.
const COMMAND_TREE_MAX_DEPTH: usize = 24;

/// What the hurt put on the wire, collected as the packets arrive.
///
/// The three packets a `/damage` produces are not answers to it: two are
/// broadcasts the command makes on its way through `hurtServer` and the third
/// is sent by the next `ServerPlayer.doTick`. None of them is a chat message,
/// so the fence the rest of this module waits on says nothing about them, and
/// they are picked up wherever in the stream they land instead.
#[derive(Debug, Default)]
struct DamageLegObserver {
    /// Whether the landing `/damage` has been sent.
    ///
    /// Everything before it belongs to the join: `doTick` sends a first
    /// `ClientboundSetHealthPacket` as soon as a session has a player, because
    /// `lastSentHealth` starts at a value no health can equal. Reporting that
    /// one would report the join instead of the hurt.
    armed: bool,
    first_health_update: Option<PlayerHealthObservation>,
    damage_events: Vec<DamageEventObservation>,
    sounds_heard: Vec<SoundObservation>,
}

impl DamageLegObserver {
    /// Starts recording, which the caller does before the landing `/damage`
    /// goes on the wire rather than after it is answered.
    fn arm(&mut self) {
        self.armed = true;
    }

    /// Whether every packet the hurt owes has arrived.
    ///
    /// The sound is not waited for and does not need to be:
    /// `LivingEntity.hurtServer` plays it at :1255, inside the same call that
    /// broadcast the damage event at :1219, so a report taken once the damage
    /// event has arrived already holds it — and an empty sound list is then a
    /// server that never sent one rather than a report taken too early. The
    /// health update is what has to be waited for, because `doTick` sends it on
    /// the tick after the command rather than inside it.
    fn settled(&self) -> bool {
        self.first_health_update.is_some() && !self.damage_events.is_empty()
    }

    /// Records what one applied packet produced.
    ///
    /// `sounds_before` is how long `positioned_sounds` was before the packet
    /// was applied, and `from_the_wire` whether the packet was the sound packet
    /// itself; together they separate the sounds a server sent from the ones
    /// this client made for itself. `WorldStore` produces one of the latter off
    /// every damage event that names the local player, because vanilla's
    /// `LocalPlayer` plays its own hurt sound out of `handleDamageEvent`.
    fn record(
        &mut self,
        probe: &ProbeContext,
        packet: ObservedDamagePacket,
        from_the_wire: bool,
        sounds_before: usize,
    ) {
        if !self.armed {
            return;
        }
        match packet {
            ObservedDamagePacket::Health(health) => {
                self.first_health_update
                    .get_or_insert(PlayerHealthObservation {
                        health: health.health,
                        food: health.food,
                        saturation: health.saturation,
                    });
            }
            ObservedDamagePacket::Event(event) => {
                self.damage_events.push(DamageEventObservation {
                    at_local_player: probe.world.local_player_id() == Some(event.entity_id),
                    source_type_id: event.source_type_id,
                    source_cause_id: event.source_cause_id,
                    source_direct_id: event.source_direct_id,
                    source_position: event
                        .source_position
                        .map(|position| [position.x, position.y, position.z]),
                });
            }
            ObservedDamagePacket::Other => {}
        }
        if !from_the_wire {
            return;
        }
        let here = probe
            .world
            .local_player_pose()
            .map(|pose| [pose.position.x, pose.position.y, pose.position.z])
            .unwrap_or_default();
        for sound in &probe.positioned_sounds[sounds_before.min(probe.positioned_sounds.len())..] {
            self.sounds_heard.push(sound_observation(sound, here));
        }
    }

    fn report(self) -> OperatorDamageReport {
        OperatorDamageReport {
            first_health_update: self.first_health_update,
            damage_events: self.damage_events,
            sounds_heard: self.sounds_heard,
        }
    }
}

/// The part of one clientbound packet the damage leg reads, taken before the
/// packet is applied and moved.
#[derive(Debug, Clone, Copy)]
enum ObservedDamagePacket {
    Health(packets::PlayerHealth),
    Event(packets::DamageEvent),
    Other,
}

impl ObservedDamagePacket {
    fn of(packet: &packets::PlayClientbound) -> Self {
        match packet {
            packets::PlayClientbound::SetHealth(health) => Self::Health(*health),
            packets::PlayClientbound::DamageEvent(event) => Self::Event(*event),
            _ => Self::Other,
        }
    }
}

/// Joins as the operator, runs the command list, and reports what came back.
///
/// Returns the report together with the serverbound play ids the session put
/// on the wire, which the caller folds into the replay's driven set the same
/// way it folds the peer's and the refusal sessions'.
pub(super) async fn run_operator_session(
    options: &ConnectionOptions,
    known_packs_manager: &Option<Arc<KnownPacksManager>>,
) -> Result<(OperatorSessionReport, Vec<i32>)> {
    let mut probe = connect_operator_session(options, known_packs_manager).await?;
    let mut damage = DamageLegObserver::default();
    timeout(
        OPERATOR_JOIN_TIMEOUT,
        join_operator_session(&mut probe, &mut damage),
    )
    .await
    .context("the operator session never finished joining")??;

    let command_tree = project_command_tree(probe.world.commands())?;
    timeout(
        OPERATOR_COMMAND_TIMEOUT,
        settle_after_join(&mut probe, &mut damage),
    )
    .await
    .context("the operator session was never answered for its first command")??;
    let mut commands = Vec::with_capacity(OPERATOR_PROBE_COMMANDS.len());
    for command in OPERATOR_PROBE_COMMANDS {
        if *command == OPERATOR_DAMAGE_INVULNERABLE_COMMAND {
            timeout(
                OPERATOR_COMMAND_TIMEOUT,
                settle_join_invulnerability(&mut probe, &mut damage),
            )
            .await
            .context("the operator session was never carried past its join invulnerability")??;
        }
        if *command == OPERATOR_DAMAGE_COMMAND {
            damage.arm();
        }
        let messages = timeout(
            OPERATOR_COMMAND_TIMEOUT,
            run_one_command(&mut probe, command, &mut damage),
        )
        .await
        .with_context(|| format!("the operator session was never answered for /{command}"))??;
        commands.push(CommandFeedbackObservation {
            command: (*command).to_string(),
            messages,
        });
    }
    // The health update is owed by the tick after the command, so it is still
    // in flight when the list ends however fast the list ran.
    timeout(
        OPERATOR_COMMAND_TIMEOUT,
        settle_damage_leg(&mut probe, &mut damage),
    )
    .await
    .with_context(|| {
        let answered = commands
            .iter()
            .filter(|entry| {
                entry.command.starts_with("damage ") || entry.command.starts_with("data ")
            })
            .map(|entry| format!("/{} answered {:?}", entry.command, entry.messages))
            .collect::<Vec<_>>()
            .join("; ");
        format!(
            "the operator session was never told what /{OPERATOR_DAMAGE_COMMAND} did ({answered})"
        )
    })??;

    let driven = probe.conn.serverbound_play_ids();
    let report = OperatorSessionReport {
        username: OPERATOR_PROBE_USERNAME.to_string(),
        commands,
        command_tree,
        damage: damage.report(),
    };
    Ok((report, driven))
}

/// Pumps the connection until the join invulnerability window has closed.
///
/// The clock it reads is the server's own: `MinecraftServer.tickChildren`
/// broadcasts `ClientboundSetTimePacket` every twentieth tick
/// (`MinecraftServer.forceGameTimeSynchronization`) and the packet carries `getGameTime()`, so counting from the
/// first one this session saw bounds the wait in the ticks
/// [`CLIENT_LOADED_TIMEOUT_TICKS`] is counted in rather than in wall clock. The
/// first one seen is at most a broadcast interval after the join, which makes
/// this an over-wait rather than a short one.
async fn settle_join_invulnerability(
    probe: &mut ProbeContext,
    damage: &mut DamageLegObserver,
) -> Result<()> {
    let mut play_bundle = PlayPacketBundleAssembler::default();
    let joined_at = loop {
        if let Some(time) = probe.world.world_time() {
            break time.game_time;
        }
        pump_once(probe, &mut play_bundle, damage)
            .await?
            .context("the operator session ended before the server told it the time")?;
    };
    loop {
        let now = probe
            .world
            .world_time()
            .map(|time| time.game_time)
            .unwrap_or(joined_at);
        if now.saturating_sub(joined_at) >= CLIENT_LOADED_TIMEOUT_TICKS {
            return Ok(());
        }
        pump_once(probe, &mut play_bundle, damage)
            .await?
            .context("the operator session ended inside its join invulnerability")?;
    }
}

/// Pumps the connection until every packet the hurt owes has arrived.
///
/// Sends nothing: the session is already holding whatever the command list left
/// in flight, and `read_packet_or_eof_or_send_play_tick` keeps the session
/// alive on its own while it waits. A server that never sends the health update
/// expires the caller's timeout, which is the same fail-closed answer a server
/// that stops answering commands gets.
async fn settle_damage_leg(probe: &mut ProbeContext, damage: &mut DamageLegObserver) -> Result<()> {
    let mut play_bundle = PlayPacketBundleAssembler::default();
    while !damage.settled() {
        pump_once(probe, &mut play_bundle, damage)
            .await?
            .context("the operator session ended before the hurt was reported")?;
    }
    Ok(())
}

/// Opens the session's own connection and starts its login.
async fn connect_operator_session(
    options: &ConnectionOptions,
    known_packs_manager: &Option<Arc<KnownPacksManager>>,
) -> Result<ProbeContext> {
    let identity = ClientIdentity::offline(OPERATOR_PROBE_USERNAME);
    let mut session_options = ConnectionOptions::offline_with_identity(&options.address, &identity)
        .context("the operator session needs an offline identity")?;
    session_options.host = options.host.clone();
    session_options.port = options.port;
    session_options.timeout = options.timeout;
    session_options.accepted_code_of_conduct_hash = options.accepted_code_of_conduct_hash;
    session_options.client_information = options.client_information.clone();
    session_options.server_resource_pack_policy = options.server_resource_pack_policy;

    let mut probe =
        ProbeContext::new(RawConnection::connect(&session_options.address, None).await?);
    probe.known_packs_manager = known_packs_manager.clone();
    probe.accepted_code_of_conduct_hash = session_options.accepted_code_of_conduct_hash;
    probe.client_information = session_options.client_information.clone();

    probe
        .session
        .start_login(
            &session_options.host,
            session_options.port,
            ClientIntent::Login,
            &session_options.username,
            session_options.profile_id,
        )
        .await?;
    Ok(probe)
}

/// Pumps the connection until the session is a loaded player holding a
/// command tree.
///
/// Both halves are required. The tree arrives during the join, from
/// `PlayerList.sendPlayerPermissionLevel` — the same
/// method that announces the session's permission level — so a session that is
/// loaded and holds no tree is one whose server published neither, and every
/// command below is parsed against it.
async fn join_operator_session(
    probe: &mut ProbeContext,
    damage: &mut DamageLegObserver,
) -> Result<()> {
    let mut play_bundle = PlayPacketBundleAssembler::default();
    while !(probe.player_loaded_sent && !probe.world.commands().nodes.is_empty()) {
        pump_once(probe, &mut play_bundle, damage)
            .await?
            .context("the operator session ended before it finished joining")?;
    }
    Ok(())
}

/// Reads and applies one clientbound packet, or reports end of stream.
/// Pumps one packet through [`ClientProtocolSession`]; shared with
/// `read_packet_or_eof_or_send_play_tick` for play tick liveness.
async fn pump_once(
    probe: &mut ProbeContext,
    play_bundle: &mut PlayPacketBundleAssembler,
    damage: &mut DamageLegObserver,
) -> Result<Option<()>> {
    let Some((packet_id, payload)) = probe.session.read_packet_or_eof_or_send_play_tick().await?
    else {
        return Ok(None);
    };
    match probe.session.decode_packet(packet_id, &payload)? {
        DecodedClientboundPacket::Login(packet) => {
            probe.handle_login_packet(packet).await?;
        }
        DecodedClientboundPacket::Configuration(packet) => {
            probe.handle_configuration_packet(packet).await?;
        }
        DecodedClientboundPacket::Play(packet) => {
            if let Some(batch) = play_bundle.push(packet)? {
                for packet in batch {
                    let observed = ObservedDamagePacket::of(&packet);
                    let from_the_wire = matches!(packet, packets::PlayClientbound::Sound(_));
                    let sounds_before = probe.positioned_sounds.len();
                    probe.handle_play_packet(packet).await?;
                    damage.record(probe, observed, from_the_wire, sounds_before);
                }
            }
        }
    }
    Ok(Some(()))
}

/// Runs the fence once and throws away everything it collects.
///
/// A join is not silent: `PlayerList.placeNewPlayer`
/// broadcasts `multiplayer.player.joined` to everyone, the joining session
/// included, and does it at PlayerList — just before the teleport
/// whose position packet is what says the session is loaded. Vanilla therefore has
/// it applied by the time the join finishes, but "therefore" is not an
/// observation, so this settles the session against the fence instead of
/// assuming the order. It also proves the fence itself is answered before the
/// list starts relying on it.
async fn settle_after_join(probe: &mut ProbeContext, damage: &mut DamageLegObserver) -> Result<()> {
    run_one_command_inner(probe, None, damage).await.map(|_| ())
}

/// Sends one command, then the fence, and returns what the command answered.
///
/// Everything that arrives before the fence's own refusal belongs to the
/// command: both requests reach the dispatcher through `tryHandleChat`
/// which posts each to the
/// server's task queue, so they run in the order they were sent. A command
/// that answers nothing is reported as answering nothing rather than waited
/// out on a clock.
///
/// New messages are picked out by `display_sequence` rather than by their
/// position, because the client drops the oldest once it is holding a hundred
/// and an index into that list does not survive it.
async fn run_one_command(
    probe: &mut ProbeContext,
    command: &str,
    damage: &mut DamageLegObserver,
) -> Result<Vec<CommandMessageObservation>> {
    run_one_command_inner(probe, Some(command), damage).await
}

/// The body of [`run_one_command`], with the observed command made optional so
/// the settle round can send the fence on its own.
async fn run_one_command_inner(
    probe: &mut ProbeContext,
    command: Option<&str>,
    damage: &mut DamageLegObserver,
) -> Result<Vec<CommandMessageObservation>> {
    let named = command.unwrap_or(OPERATOR_PROBE_FENCE_COMMAND);
    let mut seen = probe
        .world
        .client_chat()
        .messages
        .iter()
        .map(|message| message.display_sequence)
        .max()
        .unwrap_or(0);
    if let Some(command) = command {
        send_command(probe, command).await?;
    }
    send_command(probe, OPERATOR_PROBE_FENCE_COMMAND).await?;

    let mut play_bundle = PlayPacketBundleAssembler::default();
    let mut messages = Vec::new();
    loop {
        let arrived = probe
            .world
            .client_chat()
            .messages
            .iter()
            .filter(|message| message.display_sequence > seen)
            .map(|message| (message.display_sequence, message.content_component.clone()))
            .collect::<Vec<_>>();
        for (sequence, component) in arrived {
            seen = seen.max(sequence);
            let component = component
                .with_context(|| format!("/{named} was answered by a message with no component"))?;
            let observation = observe_command_message(&component)?;
            if observation.mentions_translation_key(OPERATOR_PROBE_FENCE_TRANSLATION_KEY) {
                return Ok(messages);
            }
            messages.push(observation.observation);
        }
        pump_once(probe, &mut play_bundle, damage)
            .await?
            .with_context(|| format!("the operator session ended while running /{named}"))?;
    }
}

/// Puts one command on the wire as an unsigned chat command.
///
/// Unsigned is what every command here needs. Most of them declare no signable
/// argument at all, and the two that do — `/msg` and `/teammsg`, which both
/// take `MessageArgument.message()` — are answered anyway:
/// `performUnsignedChatCommand`
/// only refuses a request that omits signatures while `enforceSecureProfile()`
/// is on, and on a dedicated server that is `enforceSecureProfile && onlineMode
/// && canValidateProfileKeys`. Both of the
/// differential's servers run offline, so both take the `else` branch.
async fn send_command(probe: &mut ProbeContext, command: &str) -> Result<()> {
    let (id, payload) = packets::encode_play_chat_command(&packets::ChatCommand {
        command: command.to_string(),
    });
    probe.conn.send_packet(id, &payload).await?;
    Ok(())
}

/// One observed message plus the translation keys it is built from.
pub(super) struct ObservedCommandMessage {
    pub(super) observation: CommandMessageObservation,
    translation_keys: BTreeSet<bbb_protocol::JavaString>,
}

impl ObservedCommandMessage {
    pub(super) fn mentions_translation_key(&self, key: &str) -> bool {
        bbb_protocol::JavaString::with_utf16(key, |units| self.translation_keys.contains(units))
    }
}

/// Projects one feedback component into the form the differential compares.
pub(super) fn observe_command_message(
    component: &NbtSummaryValue,
) -> Result<ObservedCommandMessage> {
    let canonical = canonical_component_codec_value(component)
        .map_err(|error| anyhow!("command feedback is not a valid component: {error}"))?;
    let canonical = canonical_codec_value_to_nbt(canonical);
    let mut translation_keys = BTreeSet::new();
    collect_translation_keys(&canonical, &mut translation_keys);
    Ok(ObservedCommandMessage {
        observation: CommandMessageObservation {
            translate: root_translation_key(&canonical),
            snbt: canonical_snbt_string(&canonical),
        },
        translation_keys,
    })
}

/// The `translate` key of a component's own root, ignoring its children.
fn root_translation_key(value: &NbtSummaryValue) -> Option<String> {
    let NbtSummaryValue::Compound(entries) = value else {
        return None;
    };
    entries.iter().find_map(|entry| match &entry.value {
        NbtSummaryValue::String(key) if entry.name == "translate" => Some(key.to_display_string()),
        _ => None,
    })
}

/// Every `translate` key anywhere in a component, root and children alike.
fn collect_translation_keys(
    value: &NbtSummaryValue,
    into: &mut BTreeSet<bbb_protocol::JavaString>,
) {
    match value {
        NbtSummaryValue::Compound(entries) => {
            for entry in entries {
                if entry.name == "translate" {
                    if let NbtSummaryValue::String(key) = &entry.value {
                        into.insert(key.clone());
                    }
                }
                collect_translation_keys(&entry.value, into);
            }
        }
        NbtSummaryValue::List(values) => {
            for value in values {
                collect_translation_keys(value, into);
            }
        }
        _ => {}
    }
}

/// Puts a canonically decoded component back into the NBT shape
/// `canonical_snbt_string` formats.
///
/// The canonical form exists to erase the choices two encoders may make freely
/// — a bare string where an object is allowed, a boolean where NBT only has
/// bytes — so this is the inverse that keeps those choices erased. Booleans
/// become bytes because that is what `NbtOps` writes them as, and an owner
/// node hands back the value its own subsystem already normalized.
fn canonical_codec_value_to_nbt(value: CanonicalComponentCodecValue<'_>) -> NbtSummaryValue {
    match value {
        CanonicalComponentCodecValue::Boolean(value) => NbtSummaryValue::Byte(i8::from(value)),
        CanonicalComponentCodecValue::Int(value) => NbtSummaryValue::Int(value),
        CanonicalComponentCodecValue::String(value) => NbtSummaryValue::String(value.into_owned()),
        CanonicalComponentCodecValue::List(values) => NbtSummaryValue::List(
            values
                .into_iter()
                .map(canonical_codec_value_to_nbt)
                .collect(),
        ),
        CanonicalComponentCodecValue::IntArray(values) => NbtSummaryValue::IntArray(values),
        CanonicalComponentCodecValue::Object(fields) => NbtSummaryValue::Compound(
            fields
                .into_iter()
                .map(|(name, value)| NbtSummaryEntry {
                    name: name.into(),
                    value: canonical_codec_value_to_nbt(value),
                })
                .collect(),
        ),
        CanonicalComponentCodecValue::Nbt(value) => clone_nbt_with_last_wins_compounds(value),
        CanonicalComponentCodecValue::Owner(_, value) => value.clone(),
    }
}

/// Projects the published tree into the two comparable halves.
///
/// It also decides whether the session is really an operator. Every root
/// [`OPERATOR_PROBE_COMMANDS`] uses is gated behind a permission the fixture is
/// supposed to have granted, and `fillUsableCommands`
/// never sends a node the source cannot use — so a missing root is a session
/// that was never opped, and the run fails here rather than going on to
/// collect the two empty observations such a session would produce on both
/// sides.
fn project_command_tree(tree: &CommandTreeState) -> Result<OperatorCommandTreeReport> {
    let root = tree
        .nodes
        .get(usize::try_from(tree.root_index).unwrap_or(usize::MAX))
        .context("the operator session was published a tree with no root")?;
    let roots = root_children_by_name(tree, root)?;

    let mut root_names = roots.keys().cloned().collect::<Vec<_>>();
    root_names.sort();

    let probed_roots = probed_root_names();
    let missing = probed_roots
        .iter()
        .filter(|name| !roots.contains_key(*name))
        .cloned()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        bail!(
            "the operator session was not published {missing:?}, so it is not an operator on this \
             server: check that its ops.json lists {OPERATOR_PROBE_USERNAME} at level 4"
        );
    }

    let paths = node_paths(tree, root)?;
    let mut probed_command_nodes = Vec::new();
    for name in &probed_roots {
        let index = roots[name];
        collect_subtree(tree, index, &paths, &mut probed_command_nodes)?;
    }
    probed_command_nodes.sort_by(|left, right| left.path.cmp(&right.path));

    Ok(OperatorCommandTreeReport {
        root_names,
        probed_roots,
        probed_command_nodes,
    })
}

/// The first word of every entry of [`OPERATOR_PROBE_COMMANDS`], sorted.
///
/// Derived from the command list rather than written out beside it, so the
/// projection covers exactly the commands the session runs and cannot drift
/// away from them.
fn probed_root_names() -> Vec<String> {
    let mut names = OPERATOR_PROBE_COMMANDS
        .iter()
        .filter_map(|command| command.split(' ').next())
        .map(str::to_string)
        .collect::<Vec<_>>();
    names.sort();
    names.dedup();
    names
}

/// The root's children, keyed by the literal name each one carries.
fn root_children_by_name(
    tree: &CommandTreeState,
    root: &CommandNodeState,
) -> Result<BTreeMap<String, i32>> {
    let mut roots = BTreeMap::new();
    for child in &root.children {
        let node = node_at(tree, *child)?;
        let name = node
            .name
            .as_ref()
            .context("a root command node arrived without a name")?;
        roots.insert(name.clone(), *child);
    }
    Ok(roots)
}

/// Every node's path from the root, indexed the way the packet indexes nodes.
///
/// Built once for the whole tree rather than per subtree, because a redirect
/// may point anywhere — `/tell` redirects onto `/msg` — and the projection
/// reports the target by path rather than by each server's own numbering.
///
/// The walk is breadth-first and visits each index once. `fillUsableCommands`
/// builds a fresh node per child and hangs it off one
/// parent, so a second route onto the same index is not something either
/// server sends; skipping it keeps a tree that does anyway from being walked
/// forever, and the path a node keeps is then the shortest one, which both
/// sides derive the same way.
fn node_paths(tree: &CommandTreeState, root: &CommandNodeState) -> Result<Vec<Option<String>>> {
    let mut paths = vec![None; tree.nodes.len()];
    let mut pending = root
        .children
        .iter()
        .map(|child| (*child, String::new(), 0usize))
        .collect::<VecDeque<_>>();
    while let Some((index, prefix, depth)) = pending.pop_front() {
        if depth >= COMMAND_TREE_MAX_DEPTH {
            bail!("the published command tree is deeper than {COMMAND_TREE_MAX_DEPTH} nodes");
        }
        let slot = usize::try_from(index)
            .ok()
            .filter(|slot| *slot < paths.len())
            .context("a command node child points outside the tree")?;
        if paths[slot].is_some() {
            continue;
        }
        let node = node_at(tree, index)?;
        let name = node
            .name
            .as_ref()
            .context("a named command node arrived without a name")?;
        let path = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix} {name}")
        };
        paths[slot] = Some(path.clone());
        for child in &node.children {
            pending.push_back((*child, path.clone(), depth + 1));
        }
    }
    Ok(paths)
}

/// Appends the projection of one node and everything under it.
///
/// Iterative and visit-once for the same reason [`node_paths`] is: the walk
/// has to finish on whatever tree arrived, not only on the tree a correct
/// server sends.
fn collect_subtree(
    tree: &CommandTreeState,
    index: i32,
    paths: &[Option<String>],
    into: &mut Vec<CommandTreeNodeObservation>,
) -> Result<()> {
    let mut seen = BTreeSet::new();
    let mut pending = vec![index];
    while let Some(index) = pending.pop() {
        if !seen.insert(index) {
            continue;
        }
        let node = node_at(tree, index)?;
        let redirect = match node.redirect {
            Some(redirect) => Some(path_at(paths, redirect)?),
            None => None,
        };
        into.push(CommandTreeNodeObservation {
            path: path_at(paths, index)?,
            kind: match node.node_type {
                CommandNodeKindState::Root => "root",
                CommandNodeKindState::Literal => "literal",
                CommandNodeKindState::Argument => "argument",
            }
            .to_string(),
            executable: node.executable,
            restricted: node.restricted,
            parser: node.parser.as_ref().map(|parser| parser.name.clone()),
            suggestions: node.suggestions.clone(),
            redirect,
        });
        pending.extend(node.children.iter().copied());
    }
    Ok(())
}

fn node_at(tree: &CommandTreeState, index: i32) -> Result<&CommandNodeState> {
    tree.nodes
        .get(usize::try_from(index).unwrap_or(usize::MAX))
        .context("a command node index points outside the tree")
}

fn path_at(paths: &[Option<String>], index: i32) -> Result<String> {
    paths
        .get(usize::try_from(index).unwrap_or(usize::MAX))
        .and_then(Option::clone)
        .context("a command node has no path from the root")
}

#[cfg(test)]
mod tests {
    use bbb_client_world::CommandArgumentParserState;

    use super::*;
    use crate::types::OPERATOR_REGENERATION_OFF_COMMAND;

    fn compound(entries: &[(&str, NbtSummaryValue)]) -> NbtSummaryValue {
        NbtSummaryValue::Compound(
            entries
                .iter()
                .map(|(name, value)| NbtSummaryEntry {
                    name: (*name).into(),
                    value: value.clone(),
                })
                .collect(),
        )
    }

    fn translatable(key: &str) -> NbtSummaryValue {
        compound(&[("translate", NbtSummaryValue::string(key))])
    }

    /// `CommandSourceStack.sendFailure`:
    /// `Component.empty().append(message).withStyle(ChatFormatting.RED)`.
    fn refusal(message: NbtSummaryValue) -> NbtSummaryValue {
        compound(&[
            ("text", NbtSummaryValue::string("")),
            ("extra", NbtSummaryValue::List(vec![message])),
            ("color", NbtSummaryValue::string("red")),
        ])
    }

    fn literal(name: &str, children: Vec<i32>) -> CommandNodeState {
        CommandNodeState {
            node_type: CommandNodeKindState::Literal,
            flags: 0,
            children,
            redirect: None,
            name: Some(name.to_string()),
            parser: None,
            suggestions: None,
            executable: false,
            restricted: true,
        }
    }

    /// A tree carrying one node per root [`OPERATOR_PROBE_COMMANDS`] uses,
    /// which is the smallest tree the projection accepts.
    fn minimal_operator_tree() -> CommandTreeState {
        let mut nodes = vec![CommandNodeState {
            node_type: CommandNodeKindState::Root,
            flags: 0,
            children: Vec::new(),
            redirect: None,
            name: None,
            parser: None,
            suggestions: None,
            executable: false,
            restricted: false,
        }];
        for (index, name) in probed_root_names().into_iter().enumerate() {
            nodes.push(literal(&name, Vec::new()));
            nodes[0].children.push(index as i32 + 1);
        }
        let mut tree = CommandTreeState::default();
        tree.nodes = nodes;
        tree
    }

    #[test]
    fn every_probed_command_names_only_the_operators_own_session() {
        // The commands that take a player must name this session and no other.
        // Naming the main replay's session would move the very things it pins
        // — its permission level and its two refusals — and naming a session
        // that is not connected would make the answer depend on each server's
        // own name cache rather than on the dispatcher.
        const TAKES_A_PLAYER: &[&str] = &["op", "deop", "pardon", "ban", "kick"];
        for command in OPERATOR_PROBE_COMMANDS
            .iter()
            .chain(std::iter::once(&OPERATOR_PROBE_FENCE_COMMAND))
        {
            let mut words = command.split(' ');
            let root = words.next().expect("a command has a root");
            if !TAKES_A_PLAYER.contains(&root) {
                continue;
            }
            assert_eq!(
                words.next(),
                Some(OPERATOR_PROBE_USERNAME),
                "/{command} names a player other than the operator's own session"
            );
            assert_eq!(words.next(), None, "/{command} names more than one player");
        }
        // And the list really does reach the player-taking branch at all.
        assert!(OPERATOR_PROBE_COMMANDS
            .iter()
            .any(|command| TAKES_A_PLAYER.contains(&command.split(' ').next().unwrap_or(""))));
    }

    #[test]
    fn the_command_list_reaches_no_root_its_own_doc_comment_declares_out() {
        // Every implemented root the doc comment above
        // [`OPERATOR_PROBE_COMMANDS`] accounts for as absent, with the shortest
        // form of why. The prose and the list are one declaration, so a root
        // added below without its paragraph being withdrawn fails here rather
        // than in a differential three minutes later. `pardon` is not here: it
        // is absent from the list because it is the fence, which runs after
        // every entry.
        const EXCLUDED: &[(&str, &str)] = &[
            ("seed", "answers from the save's own seed"),
            ("stop", "would end the server under test"),
            ("kick", "would end a session under test"),
            ("ban", "would leave a ban behind and disconnect its target"),
            (
                "deop",
                "resolves its target through the server's name cache",
            ),
            (
                "me",
                "answers through a chat broadcast rather than the source",
            ),
            (
                "say",
                "answers through a chat broadcast rather than the source",
            ),
            ("reload", "does its work off the observation"),
            ("tell", "redirects to msg, which this list already runs"),
            ("w", "redirects to msg, which this list already runs"),
            ("tm", "redirects to teammsg, which this list already runs"),
        ];
        for command in OPERATOR_PROBE_COMMANDS {
            let root = command.split(' ').next().expect("a command has a root");
            if let Some((_, why)) = EXCLUDED.iter().find(|(name, _)| *name == root) {
                panic!("/{command} is out of this list: it {why}");
            }
        }
    }

    #[test]
    fn the_damage_entries_hurt_only_the_operators_own_session() {
        for command in [
            OPERATOR_DAMAGE_INVULNERABLE_COMMAND,
            OPERATOR_DAMAGE_COMMAND,
        ] {
            assert!(
                OPERATOR_PROBE_COMMANDS.contains(&command),
                "/{command} is declared but never run"
            );
            let mut words = command.split(' ');
            assert_eq!(words.next(), Some("damage"), "/{command} is not a /damage");
            // `EntityArgument.entity()` is the command's first argument
            // and the only session this list may hurt
            // is its own: the main replay's is still connected and every
            // observation it carries was taken while it was unhurt.
            assert_eq!(
                words.next(),
                Some(OPERATOR_PROBE_USERNAME),
                "/{command} hurts a session other than the operator's own"
            );
            // `by <entity>` and `by <entity> from <cause>` (`DamageCommand`) are the
            // two nodes that name a second entity. Neither is used:
            // the ids they would put in the packet are each server's own
            // numbering, so the differential could not compare them.
            let rest = words.collect::<Vec<_>>();
            assert!(
                !rest.contains(&"by") && !rest.contains(&"from"),
                "/{command} names a second entity, whose id is not comparable"
            );
        }
    }

    #[test]
    fn the_hurt_leg_runs_where_its_three_preconditions_hold() {
        let position = |needle: &str| {
            OPERATOR_PROBE_COMMANDS
                .iter()
                .position(|command| *command == needle)
                .unwrap_or_else(|| panic!("the list runs /{needle}"))
        };
        let regeneration_off = position(OPERATOR_REGENERATION_OFF_COMMAND);
        let refused = position(OPERATOR_DAMAGE_INVULNERABLE_COMMAND);
        let landed = position(OPERATOR_DAMAGE_COMMAND);
        // The regeneration rule has to be off before the hit, or the health the
        // hit produces depends on which tick it landed on
        // (ServerPlayer.tickRegeneration).
        assert!(regeneration_off < landed);
        // The refusal has to come first, because it is the entry that proves
        // the hit started from an unhurt session.
        assert!(refused < landed);
        // Both have to answer, so both have to run while the feedback gate is
        // still open.
        assert!(landed < position("gamerule send_command_feedback false"));
        // And both have to run while the session is still creative, which is
        // what makes `bypasses_invulnerability` the reason the hit lands.
        assert!(landed < position("defaultgamemode survival"));
    }

    #[test]
    fn the_feedback_gate_is_probed_from_both_sides() {
        let off = OPERATOR_PROBE_COMMANDS
            .iter()
            .position(|command| *command == "gamerule send_command_feedback false")
            .expect("the list turns the feedback gate off");
        let on = OPERATOR_PROBE_COMMANDS
            .iter()
            .position(|command| *command == "gamerule send_command_feedback true")
            .expect("the list turns the feedback gate back on");
        assert!(off < on, "the gate has to come back on after it went off");
        // Between them the list has to run both a command that succeeds — and
        // must therefore fall silent — and one that fails, which must not.
        let between = &OPERATOR_PROBE_COMMANDS[off + 1..on];
        assert!(
            between
                .iter()
                .filter(|command| **command == "save-off")
                .count()
                == 2
        );
        assert!(between.contains(&"save-on"));
        // And the list has to leave auto-save the way it found it. Replaying
        // `setAutoSave` over the list is the
        // only honest way to say so: the same command appears twice in a row
        // on purpose, and the second one changes nothing.
        let mut auto_save = true;
        let mut refused = 0usize;
        let mut changed = 0usize;
        for command in OPERATOR_PROBE_COMMANDS {
            let enable = match *command {
                "save-off" => false,
                "save-on" => true,
                _ => continue,
            };
            if auto_save == enable {
                refused += 1;
            } else {
                changed += 1;
                auto_save = enable;
            }
        }
        assert!(auto_save, "the list must leave auto-save enabled");
        assert!(
            changed > 0 && refused > 0,
            "both channels have to be reached"
        );
    }

    #[test]
    fn probed_roots_are_the_first_word_of_every_command() {
        let roots = probed_root_names();
        assert_eq!(
            roots,
            [
                "ban-ip",
                "banlist",
                "damage",
                "defaultgamemode",
                "difficulty",
                "gamerule",
                "list",
                "msg",
                "op",
                "pardon-ip",
                "save-all",
                "save-off",
                "save-on",
                "teammsg",
                "teleport",
                "tick",
                "whitelist",
            ]
        );
    }

    #[test]
    fn a_session_that_was_never_opped_fails_instead_of_reporting_nothing() {
        let mut tree = minimal_operator_tree();
        // Drop the one root that needs the highest permission, which is what a
        // server that did not consider this session an operator would do.
        let dropped = tree
            .nodes
            .iter()
            .position(|node| node.name.as_deref() == Some("save-off"))
            .expect("the minimal tree carries every probed root");
        tree.nodes[0]
            .children
            .retain(|child| *child as usize != dropped);
        let error = project_command_tree(&tree).expect_err("an unopped session must fail");
        let message = error.to_string();
        assert!(message.contains("save-off"), "{message}");
        assert!(message.contains(OPERATOR_PROBE_USERNAME), "{message}");
    }

    #[test]
    fn the_projection_keys_nodes_on_their_path_and_resolves_redirects() {
        let mut tree = minimal_operator_tree();
        let gamerule = tree
            .nodes
            .iter()
            .position(|node| node.name.as_deref() == Some("gamerule"))
            .expect("the minimal tree carries every probed root") as i32;
        let rule = tree.nodes.len() as i32;
        tree.nodes
            .push(literal("send_command_feedback", vec![rule + 1]));
        tree.nodes.push(CommandNodeState {
            node_type: CommandNodeKindState::Argument,
            flags: 0,
            children: Vec::new(),
            redirect: Some(gamerule),
            name: Some("value".to_string()),
            parser: Some(CommandArgumentParserState {
                type_id: 0,
                name: "brigadier:bool".to_string(),
                properties: Vec::new(),
            }),
            suggestions: Some("minecraft:available_sounds".to_string()),
            executable: true,
            restricted: false,
        });
        tree.nodes[gamerule as usize].children.push(rule);

        let report = project_command_tree(&tree).expect("the tree carries every probed root");
        assert_eq!(report.probed_roots, probed_root_names());
        assert!(report.root_names.windows(2).all(|pair| pair[0] < pair[1]));
        let paths = report
            .probed_command_nodes
            .iter()
            .map(|node| node.path.as_str())
            .collect::<Vec<_>>();
        assert!(paths.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(paths.contains(&"gamerule send_command_feedback"));

        let value = report
            .probed_command_nodes
            .iter()
            .find(|node| node.path == "gamerule send_command_feedback value")
            .expect("the argument node is projected");
        assert_eq!(value.kind, "argument");
        assert!(value.executable);
        assert!(!value.restricted);
        assert_eq!(value.parser.as_deref(), Some("brigadier:bool"));
        assert_eq!(
            value.suggestions.as_deref(),
            Some("minecraft:available_sounds")
        );
        assert_eq!(value.redirect.as_deref(), Some("gamerule"));
    }

    #[test]
    fn the_projection_leaves_out_roots_the_command_list_never_runs() {
        let mut tree = minimal_operator_tree();
        // `seed` is the root the command list is never allowed to run, so it
        // is the one a server may publish without the equality half growing.
        let extra = tree.nodes.len() as i32;
        tree.nodes.push(literal("seed", Vec::new()));
        tree.nodes[0].children.push(extra);

        let report = project_command_tree(&tree).expect("the tree carries every probed root");
        assert!(
            report.root_names.contains(&"seed".to_string()),
            "the root names are the whole set the session was published"
        );
        assert!(
            !report
                .probed_command_nodes
                .iter()
                .any(|node| node.path == "seed"),
            "the equality half covers only what the command list runs"
        );
    }

    #[test]
    fn a_refusal_carries_no_root_translate_and_a_success_does() {
        let refused = observe_command_message(&refusal(translatable("commands.op.failed")))
            .expect("the refusal is a valid component");
        assert_eq!(refused.observation.translate, None);
        assert!(refused.observation.snbt.contains("commands.op.failed"));
        assert!(
            refused.observation.snbt.contains("red"),
            "the empty red root is what says a refusal is a refusal: {}",
            refused.observation.snbt
        );

        let succeeded = observe_command_message(&translatable("commands.save.disabled"))
            .expect("the success is a valid component");
        assert_eq!(
            succeeded.observation.translate.as_deref(),
            Some("commands.save.disabled")
        );
    }

    #[test]
    fn the_fence_is_recognised_through_the_red_wrapper_it_arrives_in() {
        let fence =
            observe_command_message(&refusal(translatable(OPERATOR_PROBE_FENCE_TRANSLATION_KEY)))
                .expect("the fence's refusal is a valid component");
        assert!(fence.mentions_translation_key(OPERATOR_PROBE_FENCE_TRANSLATION_KEY));
        // And nothing else the list runs is mistaken for it.
        let other = observe_command_message(&refusal(translatable("commands.save.alreadyOff")))
            .expect("the refusal is a valid component");
        assert!(!other.mentions_translation_key(OPERATOR_PROBE_FENCE_TRANSLATION_KEY));
    }

    #[test]
    fn the_canonical_form_erases_the_shorthands_two_encoders_may_choose_between() {
        // `Component.empty().append(x)` may serialize its one sibling as a
        // list or, where the codec allows it, as the value itself; a plain
        // string is the shorthand for `{"text": ...}`. Neither choice is a
        // difference between two servers, so neither may reach the snbt.
        let listed = compound(&[
            ("text", NbtSummaryValue::string("")),
            (
                "extra",
                NbtSummaryValue::List(vec![NbtSummaryValue::string("bbb")]),
            ),
        ]);
        let spelled = compound(&[
            ("text", NbtSummaryValue::string("")),
            (
                "extra",
                NbtSummaryValue::List(vec![compound(&[("text", NbtSummaryValue::string("bbb"))])]),
            ),
        ]);
        let listed = observe_command_message(&listed).expect("valid component");
        let spelled = observe_command_message(&spelled).expect("valid component");
        assert_eq!(listed.observation.snbt, spelled.observation.snbt);
    }

    #[test]
    fn the_snbt_keeps_the_arguments_the_flattened_text_drops() {
        // `commands.list.players` is the shape that makes this necessary: its
        // whole answer is its arguments, and two of the three are numbers,
        // which `styled_runs_summary_text` does not keep.
        let message = compound(&[
            (
                "translate",
                NbtSummaryValue::string("commands.list.players"),
            ),
            (
                "with",
                NbtSummaryValue::List(vec![
                    NbtSummaryValue::Int(2),
                    NbtSummaryValue::Int(20),
                    NbtSummaryValue::string("bbb-probe"),
                ]),
            ),
        ]);
        let observed = observe_command_message(&message).expect("valid component");
        assert!(
            observed.observation.snbt.contains('2'),
            "{}",
            observed.observation.snbt
        );
        assert!(
            observed.observation.snbt.contains("20"),
            "{}",
            observed.observation.snbt
        );
        assert!(
            observed.observation.snbt.contains("bbb-probe"),
            "{}",
            observed.observation.snbt
        );
    }
}
