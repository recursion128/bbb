use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
    time::Duration,
};

use anyhow::{anyhow, bail, Context, Result};
use bbb_client_assets::KnownPacksManager;
use bbb_client_world::{
    block_name_is_air, BlockPos, ChunkPos, LevelEventSoundRandomState, LocalPlayerPoseState,
    SoundEventState, WorldStore,
};
use bbb_protocol::{
    data_component_types::VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID,
    item_types::VANILLA_ITEM_STONE_ID,
    packets::{
        self, BlockHitResult, ClientIntent, DataComponentPatchSummary, Direction, InteractionHand,
        ItemStackSummary, PlayerAction, PlayerActionKind, SetCreativeModeSlot, UseItemOn,
    },
    player_command_actions::PlayerCommandAction,
    resource_pack_response_actions::ResourcePackResponseAction,
    sound_event_types::vanilla_sound_event_resource_id,
    NbtSummaryEntry, NbtSummaryValue,
};
use tokio::time::{timeout, Instant};
use uuid::Uuid;

use crate::types::{
    BlockEventObservation, ChunkDeliveryReport, ChunkProbeSummary, CommandSuggestionObservation,
    ConfigurationCatalogReport, DimensionTransitionsReport, HoldReport, InteractionReplayReport,
    OperatorSessionReport, OutwardWalkReport, PeerObservationReport, PeerSharedFlagsReport,
    ProbeOptions, ProbeReport, RefusalObservation, RefusalProbeReport, SessionAuthorityReport,
    SessionLevelReport, SoundAtBlockObservation, SoundObservation, SpawnedEntityObservation,
    COMMAND_BLOCK_NOT_ALLOWED_KEY, ENTITY_RUNTIME_PROBE_USERNAME, INTERACTION_ANVIL_NAME,
    INTERACTION_BOOK_INVENTORY_SLOT, INTERACTION_BOOK_MENU_SLOT, INTERACTION_BOOK_PAGE,
    INTERACTION_BOOK_TITLE, INTERACTION_CHILD_ADVANCEMENT_TAB, INTERACTION_CLICKED_MENU_SLOT,
    INTERACTION_CLICKED_STACK_COUNT, INTERACTION_CUSTOM_CLICK_ACTION, INTERACTION_DROP_BLOCK_NAME,
    INTERACTION_DROP_CELL_OFFSET, INTERACTION_DROP_INVENTORY_SLOT, INTERACTION_DROP_ITEM_COUNT,
    INTERACTION_DROP_MENU_SLOT, INTERACTION_HELD_STACK_COUNT,
    INTERACTION_NOTE_BLOCK_INVENTORY_SLOT, INTERACTION_NOTE_BLOCK_MENU_SLOT,
    INTERACTION_NOTE_BLOCK_NAME, INTERACTION_PICK_PARKED_SLOT, INTERACTION_PING_TIME,
    INTERACTION_PONG_ID, INTERACTION_REDSTONE_WIRE_INVENTORY_SLOT,
    INTERACTION_REDSTONE_WIRE_ITEM_NAME, INTERACTION_REDSTONE_WIRE_MENU_SLOT,
    INTERACTION_REQUESTED_DIFFICULTY, INTERACTION_ROOT_ADVANCEMENT_TAB, INTERACTION_SIGN_LINE,
    INTERACTION_UNKNOWN_ADVANCEMENT_TAB, INTERACTION_UNRESOLVABLE_RECIPE_DISPLAY,
    INTERACTION_USE_ITEM_SEQUENCE, INTERACTION_USE_ITEM_UNWRAPPED_Y_ROT,
    INVALID_COMMAND_SIGNATURE_TRANSLATION_KEY, LIST_COMMAND_TRANSLATION_KEY,
    MALFORMED_BODY_MESSAGE, MALFORMED_BODY_SALT, MALFORMED_BODY_TIMESTAMP_MILLIS,
    MALFORMED_NBT_ENTRY_NAME, MALFORMED_NBT_SLOT, OPERATOR_PROBE_USERNAME, OUTWARD_WALK_CHUNKS,
    PERMISSION_QUERY_TRANSACTION_ID, PERMISSION_REFUSED_COMMAND, PERMISSION_REFUSED_GAME_RULE,
    PERMISSION_REFUSED_STRUCTURE, REFUSED_COOKIE_KEY, REFUSED_KEEP_ALIVE_ID,
};
use bbb_client_net::{
    ClientIdentity, ClientProtocolSession, ConnectionOptions, ConnectionState,
    DecodedClientboundPacket, PlayPacketBundleAssembler, RawConnection,
};

const INTERACTION_CHAT_MESSAGE: &str = "bbb fixture replay";
/// The command the replay runs, without the leading slash that
/// `ServerboundChatCommandPacket` strips. `/list` needs no permission and its
/// reply is a pure function of the connected players, so it is stable across
/// both servers.
const INTERACTION_LIST_COMMAND: &str = "list";
/// The signature material the signed form of that command carries. The
/// timestamp and salt only ever reach `SignedMessageBody`, which an offline
/// server's `SignedMessageChain.Decoder.unsigned` ignores, so any fixed pair
/// describes both sides. The argument name is one no command in reach reads,
/// which is what makes the second request the mismatch case.
const INTERACTION_SIGNED_COMMAND_TIMESTAMP_MILLIS: i64 = 1_700_000_000_000;
const INTERACTION_SIGNED_COMMAND_SALT: i64 = 0x0bbb_0bbb;
const INTERACTION_SIGNED_COMMAND_ARGUMENT: &str = "message";
const INTERACTION_SIGNED_COMMAND_SIGNATURE_BYTE: u8 = 0x7f;
/// The two partial commands the replay asks to complete. The client keeps only
/// the answer to the request it is still waiting on
/// (`WorldStore::apply_command_suggestions`), so each wave asks exactly once.
/// Both are prefixes only `/list` and its one child can complete on either
/// server: nothing else either dispatcher offers a permissionless session
/// starts with them.
const INTERACTION_ROOT_SUGGESTION_COMMAND: &str = "lis";
const INTERACTION_CHILD_SUGGESTION_COMMAND: &str = "list u";
/// The brand the replay declares on `minecraft:brand`. Vanilla's play-state
/// `handleCustomPayload` is empty, so the value only has to be identical on
/// both sides.
const INTERACTION_CLIENT_BRAND: &str = "bbb";
/// A resource pack the server never pushed. `handleResourcePackResponse` looks
/// only at the action and at whether a pack is required, so a decline for an
/// unknown id is accepted by both servers.
const INTERACTION_RESOURCE_PACK_ID: uuid::Uuid = uuid::Uuid::from_u128(0x6262_6200_0000_0000);

/// The sequences the note-block leg's two uses acknowledge with.
///
/// Both sit below [`INTERACTION_USE_ITEM_SEQUENCE`], which the replay's last
/// packet carries: `ackBlockChangesUpTo` keeps the maximum, so the
/// acknowledgement the report reads still names the use in the air.
const INTERACTION_REDSTONE_BASE_PLACE_SEQUENCE: i32 = 3;
const INTERACTION_NOTE_BLOCK_PLACE_SEQUENCE: i32 = 4;
const INTERACTION_NOTE_BLOCK_TUNE_SEQUENCE: i32 = 5;

/// The sequence the redstone-wire leg's one place acknowledges with. It runs
/// between the note-block leg and the drop leg, so it counts up between them.
const INTERACTION_REDSTONE_WIRE_PLACE_SEQUENCE: i32 = 6;

/// The sequences the drop leg's two block actions acknowledge with.
///
/// They sit above the wire's and below [`INTERACTION_USE_ITEM_SEQUENCE`] for
/// the same reason: a vanilla client counts up, and the acknowledgement the
/// report reads has to stay the use in the air.
const INTERACTION_DROP_PLACE_SEQUENCE: i32 = 7;
const INTERACTION_DROP_BREAK_SEQUENCE: i32 = 8;

/// How many of the server's own ticks the drop leg's pickup may take before
/// the session stops waiting for it.
///
/// Ten times the ten `ItemEntity.setDefaultPickUpDelay` counts down
/// which is the whole of what a server that behaves
/// has to wait for: the player never moves while the wait runs, so the only
/// thing between the drop and the pickup is that delay and the tick the
/// collision check runs on. A margin that large cannot hide a difference —
/// nothing in vanilla makes a pickup ninety ticks late — and it keeps a server
/// that never picks the item up out of the probe's overall timeout, so the
/// report says which side did not take it instead of saying that something
/// hung.
const DROP_PICKUP_TICK_BUDGET: i64 = 100;

mod configuration;
mod container_menu;
mod dimension_transition;
mod entity_runtime;
mod login;
mod operator;
mod play;

struct ProbeContext {
    session: ClientProtocolSession,
    world: WorldStore,
    started_at: std::time::Instant,
    transfer_options: Option<ConnectionOptions>,
    player_loaded_sent: bool,
    level_event_sound_random: LevelEventSoundRandomState,
    unsupported_packets: usize,
    last_unsupported_packet_state: Option<ConnectionState>,
    last_unsupported_packet_id: Option<i32>,
    last_unsupported_packet_len: Option<usize>,
    known_packs_manager: Option<Arc<KnownPacksManager>>,
    interaction_replay: Option<InteractionReplayReport>,
    peer_observation: Option<PeerObservationReport>,
    session_authority: Option<SessionAuthorityReport>,
    outward_walk: Option<OutwardWalkReport>,
    /// What the operator session found. It runs on its own connection, so this
    /// is where its report is parked until the main session is finished with.
    operator_session: Option<OperatorSessionReport>,
    chunk_delivery: ChunkDeliveryObserver,
    /// Every `ClientboundSoundPacket` this session has been handed, in arrival
    /// order.
    positioned_sounds: Vec<SoundEventState>,
    /// The subset of [`positioned_sounds`] a server actually sent, in arrival
    /// order.
    ///
    /// The two are not the same list. `positioned_sounds` is everything the
    /// world model produced a positioned sound for, and some of those are the
    /// client's own work: a note block's, for one, is synthesised out of
    /// `ClientboundBlockEventPacket` rather than received. A differential that
    /// read the whole list would be comparing this client against itself and
    /// would report two servers as agreeing about a sound neither of them
    /// sent, so what a *server* owes is tracked separately — by which packet
    /// was being applied when the sound appeared.
    ///
    /// [`positioned_sounds`]: ProbeContext::positioned_sounds
    wire_positioned_sounds: Vec<SoundEventState>,
    /// One entry per block event this session has been handed, in the same
    /// order, holding how many server-sent sounds had arrived before it. The
    /// world keeps the events themselves; this is the arrival fact it cannot
    /// know, and it only tracks the session's own loop — the observations that
    /// read it are all taken there.
    block_event_sounds_before: Vec<usize>,
    /// Every entity the server has introduced to this session, frozen at the
    /// packet that introduced it.
    ///
    /// Keyed by network id so a re-used id replaces rather than duplicates,
    /// and a `BTreeMap` so the order is the ids' rather than the arrival's.
    /// See [`SpawnedEntityObservation`] for why the snapshot is taken here
    /// instead of read off the live entity list at the end.
    spawned_entities: BTreeMap<i32, SpawnedEntitySnapshot>,
    /// Every entity a `ClientboundTakeItemEntityPacket`
    /// (ClientboundTakeItemEntityPacket.java:28-32) named, by the id it named.
    ///
    /// Recorded off the packet rather than read out of the world, because
    /// applying it is what removes the entity: `apply_take_item_entity` drops
    /// the item once its stack is empty, so by the time the world could be
    /// asked, the thing to ask about is gone.
    taken_item_entities: BTreeSet<i32>,
    /// The world tick the drop leg's pickup wait may run until, once it has
    /// started.
    ///
    /// See [`ProbeContext::drop_pickup_settled`].
    drop_pickup_deadline: Option<i64>,
    configuration_capture: Option<dimension_transition::ConfigurationCapture>,
    configuration_catalog: Option<ConfigurationCatalogReport>,
    dimension_transitions: Option<DimensionTransitionsReport>,
    entity_runtime_replay: Option<crate::types::EntityRuntimeReplayReport>,
    container_menu: Option<crate::types::ContainerMenuReport>,
    first_chunk_summary: Option<ChunkProbeSummary>,
    first_chunk_center_block: Option<bbb_client_world::BlockProbe>,
}

/// One entity as the packet that introduced it described it, plus the stack
/// the metadata that followed named.
///
/// `ServerEntity.sendPairingData` sends the spawn and the non-default synched
/// data as two packets of one bundle, so the stack is never on the packet the
/// position comes from — it is filled in from the next packet that carries it
/// and then left alone.
#[derive(Debug, Clone)]
struct SpawnedEntitySnapshot {
    entity_type_id: i32,
    uuid: Uuid,
    position: [f64; 3],
    delta_movement: [f64; 3],
    item: Option<ItemStackSummary>,
}

/// Watches the batch brackets around chunk delivery.
///
/// The client already answers each finished batch; this only records whether
/// the brackets were there and whether they framed what they claimed.
#[derive(Debug, Default)]
pub(crate) struct ChunkDeliveryObserver {
    report: ChunkDeliveryReport,
    open_batch_chunks: Option<usize>,
}

impl ChunkDeliveryObserver {
    pub(crate) fn on_batch_start(&mut self) {
        // A start inside an open batch abandons the previous one; the
        // wrong-size counter below is what reports that, since the abandoned
        // batch never declares a size.
        self.open_batch_chunks = Some(0);
    }

    pub(crate) fn on_chunk(&mut self) {
        match self.open_batch_chunks.as_mut() {
            Some(chunks) => *chunks += 1,
            None => self.report.chunks_outside_batches += 1,
        }
    }

    pub(crate) fn on_batch_finished(&mut self, batch_size: i32) {
        let delivered = self.open_batch_chunks.take();
        if delivered != usize::try_from(batch_size).ok() {
            self.report.batches_with_wrong_size += 1;
        }
        self.report.completed_a_batch = true;
    }

    fn report(&self) -> ChunkDeliveryReport {
        self.report.clone()
    }
}

#[derive(Debug, Clone, Copy)]
struct InteractionReplayPlan {
    broken_block: BlockPos,
    clicked_block: BlockPos,
    placed_block: BlockPos,
    /// Where the note block goes: directly on top of [`placed_block`], the
    /// stone this replay puts there itself.
    ///
    /// It sits on the replay's own block rather than on terrain because
    /// `NoteBlock.setInstrument` reads the block below
    /// to pick the instrument, and the instrument decides both the state the
    /// differential compares and the sound `triggerEvent` plays. A
    /// terrain base would make both a function of what each server's world
    /// happens to hold at the surface; the stone makes them a function of the
    /// replay.
    ///
    /// [`placed_block`]: InteractionReplayPlan::placed_block
    note_block: BlockPos,
    /// The surface the drop leg stands its carpet on, and then breaks.
    ///
    /// A third surface rather than either of the two above, because the leg
    /// destroys it: the broken block is already air by the time this runs and
    /// has nothing to stand anything on, and the clicked block is carrying the
    /// stone and the note block whose state two later observations read.
    ///
    /// The carpet goes directly on top of it, which is what
    /// `INTERACTION_DROP_CELL_OFFSET` states: `CarpetBlock.canSurvive`
    /// holds there because the base is not empty,
    /// and breaking the base is what makes it false again.
    drop_base: BlockPos,
    /// The terrain surface the redstone-wire leg first covers with stone.
    ///
    /// A fourth surface, distinct from all three above, and for a reason none
    /// of them shares: the shape `RedStoneWireBlock.getConnectionState`
    /// answers is a function of the four
    /// cells around the wire, so the leg's answer is only the replay's own if
    /// nothing else the replay built stands beside it. The stone above this
    /// surface makes `RedStoneWireBlock.canSurvive` terrain-neutral.
    redstone_clicked_block: BlockPos,
    /// The replay-owned stone which supports the redstone dust.
    redstone_wire_base: BlockPos,
}

#[derive(Debug)]
struct ProbeDrain {
    first_chunk: Option<ChunkPos>,
    packets_after_first_chunk: usize,
    after_first_chunk_limit: usize,
}

impl ProbeDrain {
    fn new(after_first_chunk_limit: usize) -> Self {
        Self {
            first_chunk: None,
            packets_after_first_chunk: 0,
            after_first_chunk_limit,
        }
    }

    fn observe_packet(&mut self, packet_first_chunk: Option<ChunkPos>) -> bool {
        if self.first_chunk.is_some() {
            self.packets_after_first_chunk += 1;
            return self.packets_after_first_chunk >= self.after_first_chunk_limit;
        }

        self.first_chunk = packet_first_chunk;
        self.first_chunk.is_some() && self.after_first_chunk_limit == 0
    }

    fn first_chunk(&self) -> Option<ChunkPos> {
        self.first_chunk
    }
}

/// The first complete batch is the earliest stable point for replay edits.
/// Vanilla sends every chunk snapshot between the batch brackets
/// editing after only the first snapshot can
/// race the rest of that batch and leave this client holding the old state.
fn interaction_replay_ready(drain: &ProbeDrain, chunk_delivery: &ChunkDeliveryObserver) -> bool {
    drain.first_chunk().is_some() && chunk_delivery.report.completed_a_batch
}

impl std::ops::Deref for ProbeContext {
    type Target = ClientProtocolSession;

    fn deref(&self) -> &Self::Target {
        &self.session
    }
}

impl std::ops::DerefMut for ProbeContext {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.session
    }
}

impl ProbeContext {
    fn new(conn: RawConnection) -> Self {
        Self {
            session: ClientProtocolSession::new(conn),
            world: WorldStore::new(),
            started_at: std::time::Instant::now(),
            transfer_options: None,
            player_loaded_sent: false,
            level_event_sound_random: LevelEventSoundRandomState::with_seed(0),
            unsupported_packets: 0,
            last_unsupported_packet_state: None,
            last_unsupported_packet_id: None,
            last_unsupported_packet_len: None,
            known_packs_manager: None,
            interaction_replay: None,
            peer_observation: None,
            session_authority: None,
            outward_walk: None,
            operator_session: None,
            chunk_delivery: ChunkDeliveryObserver::default(),
            positioned_sounds: Vec::new(),
            wire_positioned_sounds: Vec::new(),
            block_event_sounds_before: Vec::new(),
            spawned_entities: BTreeMap::new(),
            taken_item_entities: BTreeSet::new(),
            drop_pickup_deadline: None,
            configuration_capture: None,
            configuration_catalog: None,
            dimension_transitions: None,
            entity_runtime_replay: None,
            container_menu: None,
            first_chunk_summary: None,
            first_chunk_center_block: None,
        }
    }

    fn with_connection_options(conn: RawConnection, options: ConnectionOptions) -> Self {
        let mut ctx = Self::new(conn);
        ctx.transfer_options = Some(options);
        ctx
    }

    /// Applies one play packet and records the two arrival facts the world
    /// model cannot: which positioned sounds came off the wire, and how many of
    /// them had arrived before each block event.
    ///
    /// Both are decided per packet, which is the finest grain there is. A
    /// sound packet is the only thing that produces a sound a server sent —
    /// everything else the world model emits a positioned sound for, the
    /// client produced itself — and one packet is never both a sound and a
    /// block event, so a sound from the same tick is either already counted or
    /// still to come.
    async fn apply_observed_play_packet(
        &mut self,
        packet: packets::PlayClientbound,
    ) -> Result<Option<ChunkPos>> {
        let from_the_wire = matches!(packet, packets::PlayClientbound::Sound(_));
        if let packets::PlayClientbound::TakeItemEntity(take) = &packet {
            self.taken_item_entities.insert(take.item_id);
        }
        let sounds_before = self.positioned_sounds.len();
        let first_chunk = self.handle_play_packet(packet).await?;
        if from_the_wire {
            self.wire_positioned_sounds
                .extend_from_slice(&self.positioned_sounds[sounds_before..]);
        }
        let arrived = self.world.block_events().len();
        let sounds = self.wire_positioned_sounds.len();
        while self.block_event_sounds_before.len() < arrived {
            self.block_event_sounds_before.push(sounds);
        }
        self.record_spawned_entities();
        Ok(first_chunk)
    }

    /// Snapshots every entity the world model has learned about that this
    /// session has not already recorded, and completes the stack of one it
    /// has.
    ///
    /// Running per packet is what makes the position the spawn position: the
    /// spawn packet is applied on its own, so the entity is recorded before
    /// anything that could move it has been read. Nothing is ever revisited
    /// afterwards except the stack, which arrives one packet later and can
    /// only be filled in once.
    ///
    /// `probe_entities` leaves out the session's own avatar. It never arrives
    /// as a spawn — the login names it — and everything the differential asks
    /// about it is asked of the player observations instead.
    fn record_spawned_entities(&mut self) {
        for state in self.world.probe_entities() {
            let item = item_entity_stack(&state);
            match self.spawned_entities.entry(state.id) {
                std::collections::btree_map::Entry::Vacant(slot) => {
                    slot.insert(SpawnedEntitySnapshot {
                        entity_type_id: state.entity_type_id,
                        uuid: state.uuid,
                        position: [state.position.x, state.position.y, state.position.z],
                        delta_movement: [
                            state.delta_movement.x,
                            state.delta_movement.y,
                            state.delta_movement.z,
                        ],
                        item,
                    });
                }
                std::collections::btree_map::Entry::Occupied(mut slot) => {
                    let recorded = slot.get_mut();
                    if recorded.item.is_none() {
                        recorded.item = item;
                    }
                }
            }
        }
    }

    /// Waits bounded ticks after drop completion for `ClientboundTakeItemEntityPacket`
    /// so pickup delay settles before the replay freezes.
    fn drop_pickup_settled(&mut self, report: &InteractionReplayReport) -> bool {
        let game_time = self.world.world_time().map(|time| time.game_time);
        drop_pickup_wait_ended(
            &mut self.drop_pickup_deadline,
            report.drop_finished(),
            game_time,
        )
    }

    fn record_unsupported_packet(&mut self, state: ConnectionState, packet_id: i32, len: usize) {
        self.unsupported_packets += 1;
        self.last_unsupported_packet_state = Some(state);
        self.last_unsupported_packet_id = Some(packet_id);
        self.last_unsupported_packet_len = Some(len);
    }

    fn finish(
        self,
        packets_seen: usize,
        first_chunk: ChunkPos,
        extra_serverbound_play_ids: Vec<i32>,
        refusal_probes: Option<RefusalProbeReport>,
        hold: Option<HoldReport>,
    ) -> ProbeReport {
        let world_counters = self.world.counters();
        let world_apply_errors = self.world.world_apply_error_messages();
        // Read before the store is moved into the report, and read from the
        // store rather than from the packet: the three `dimension_type_*`
        // fields are what the client resolved the wire index to.
        let session_level = self
            .world
            .level_info()
            .map(SessionLevelReport::from_level_info);
        // The peer and the refusal sessions run on their own connections, and
        // the mechanisms only they drive are exactly the ones a single session
        // cannot reach, so the replay's driven set is the union of them all.
        let serverbound_play_ids = self
            .session
            .conn
            .serverbound_play_ids()
            .into_iter()
            .chain(extra_serverbound_play_ids)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();

        ProbeReport {
            reached_state: self.session.state,
            compression_threshold: self.session.conn.compression_threshold(),
            packets_seen,
            unsupported_packets: self.unsupported_packets,
            last_unsupported_packet_state: self
                .last_unsupported_packet_state
                .map(|state| format!("{state:?}")),
            last_unsupported_packet_id: self.last_unsupported_packet_id,
            last_unsupported_packet_len: self.last_unsupported_packet_len,
            registries_seen: world_counters.registries_seen,
            registry_entries_seen: world_counters.registry_entries_seen,
            registry_entries_with_data: world_counters.registry_entries_with_data,
            registry_entry_stubs: world_counters.registry_entry_stubs,
            registry_entry_payload_bytes: world_counters.registry_entry_payload_bytes,
            registry_content_registries_tracked: world_counters.registry_content_registries_tracked,
            registry_content_packets_tracked: world_counters.registry_content_packets_tracked,
            registry_content_entries_tracked: world_counters.registry_content_entries_tracked,
            registry_duplicate_entries: world_counters.registry_duplicate_entries,
            registry_duplicate_entry_ids_tracked: world_counters
                .registry_duplicate_entry_ids_tracked,
            last_registry_data_registry: world_counters.last_registry_data_registry.clone(),
            last_registry_data_entry_count: world_counters.last_registry_data_entry_count,
            first_chunk: Some(first_chunk),
            first_chunk_summary: self.first_chunk_summary,
            first_chunk_center_block: self.first_chunk_center_block,
            outward_walk: self.outward_walk,
            interaction_replay: self.interaction_replay,
            peer_observation: self.peer_observation,
            session_authority: self.session_authority,
            refusal_probes,
            operator_session: self.operator_session,
            hold,
            session_level,
            configuration_catalog: self.configuration_catalog,
            dimension_transitions: self.dimension_transitions,
            entity_runtime_replay: self.entity_runtime_replay,
            container_menu: self.container_menu,
            chunk_delivery: self.chunk_delivery.report(),
            serverbound_play_ids,
            world_counters,
            world_apply_errors,
            world: self.world,
        }
    }
}

/// The block at a column's centre, at the height every column probe reads.
///
/// Both the spawn column and the column the outward walk moves into go through
/// here, so the two observations describe the same slice of the world rather
/// than each picking its own and being compared anyway.
fn column_center_block(chunk: ChunkPos) -> BlockPos {
    BlockPos {
        x: chunk.x * 16 + 8,
        y: COLUMN_PROBE_Y,
        z: chunk.z * 16 + 8,
    }
}

/// The height the column probes read.
///
/// Nothing derives it from either world: it is simply a height inside the
/// overworld's published range on both servers, and its only job is to be the
/// same one for every column, so a difference between two probes is a
/// difference in the world rather than in where they looked.
const COLUMN_PROBE_Y: i32 = 64;

/// What the probe is waiting for right now, readable from outside the run.
///
/// The deadline in [`run_probe`] covers the whole program, so on its own a
/// timeout says only that *something* never arrived — which of the replay's
/// legs was still waiting, and on what, is the part that identifies the fault.
/// Every leg that can block leaves a note here and the timeout reports the
/// last one.
#[derive(Clone, Default)]
struct ProbeProgress(Arc<std::sync::Mutex<String>>);

impl ProbeProgress {
    fn note(&self, waiting_for: impl Into<String>) {
        // A poisoned lock only means an earlier writer panicked. The note is
        // diagnostic, so losing one must not become a second failure.
        if let Ok(mut slot) = self.0.lock() {
            *slot = waiting_for.into();
        }
    }

    fn waiting_for(&self) -> String {
        match self.0.lock() {
            Ok(slot) if !slot.is_empty() => slot.clone(),
            _ => "an unrecorded step".to_owned(),
        }
    }
}

pub async fn run_probe(options: ProbeOptions) -> Result<ProbeReport> {
    // A requested hold is work, not a stall, so the deadline covers the program
    // and the hold rather than making a long hold look like a hung probe.
    let deadline = options.connection.timeout + options.probe_hold.unwrap_or_default();
    let progress = ProbeProgress::default();
    timeout(deadline, run_probe_inner(options, progress.clone()))
        .await
        .with_context(|| format!("probe timed out waiting for {}", progress.waiting_for()))?
}

async fn run_probe_inner(options: ProbeOptions, progress: ProbeProgress) -> Result<ProbeReport> {
    let conn_options = &options.connection;
    let connect_started = Instant::now();
    let mut probe = ProbeContext::with_connection_options(
        RawConnection::connect(&conn_options.address, None).await?,
        conn_options.clone(),
    );
    probe.known_packs_manager = options.known_packs_manager.clone();
    probe.accepted_code_of_conduct_hash = conn_options.accepted_code_of_conduct_hash;
    probe.client_information = conn_options.client_information.clone();
    if options.probe_dimension_transitions {
        if options.probe_interactions || options.probe_operator || options.probe_hold.is_some() {
            bail!("dimension transitions probe cannot be combined with replay, operator, or hold");
        }
        if options.known_packs_manager.is_some() {
            bail!("dimension transitions probe requires no local vanilla pack roots");
        }
        if options.probe_after_first_chunk_packets != 0 {
            bail!("dimension transitions probe requires the default first-chunk drain");
        }
        probe.configuration_capture = Some(Default::default());
    }
    if options.probe_entity_runtime {
        if options.probe_interactions
            || options.probe_operator
            || options.probe_dimension_transitions
            || options.probe_hold.is_some()
        {
            bail!("entity runtime probe cannot be combined with another replay or hold");
        }
        if options.known_packs_manager.is_some() {
            bail!("entity runtime probe requires no local vanilla pack roots");
        }
        if options.probe_after_first_chunk_packets != 0 {
            bail!("entity runtime probe requires the default first-chunk drain");
        }
        if conn_options.username != ENTITY_RUNTIME_PROBE_USERNAME {
            bail!("entity runtime probe requires username {ENTITY_RUNTIME_PROBE_USERNAME}");
        }
    }
    if options.probe_container_menu {
        if options.probe_interactions
            || options.probe_operator
            || options.probe_dimension_transitions
            || options.probe_entity_runtime
            || options.probe_hold.is_some()
        {
            bail!("container menu probe cannot be combined with another replay or hold");
        }
        if options.known_packs_manager.is_some() {
            bail!("container menu probe requires no local vanilla pack roots");
        }
        if options.probe_after_first_chunk_packets != 0 {
            bail!("container menu probe requires the default first-chunk drain");
        }
        // The session steps the frozen server itself, which only an operator
        // may do, and the harness lists this one name in the ops file.
        if conn_options.username != OPERATOR_PROBE_USERNAME {
            bail!("container menu probe requires username {OPERATOR_PROBE_USERNAME}");
        }
    }
    let mut packets_seen = 0usize;

    probe
        .session
        .start_login(
            &conn_options.host,
            conn_options.port,
            ClientIntent::Login,
            &conn_options.username,
            conn_options.profile_id,
        )
        .await?;

    let mut drain = ProbeDrain::new(options.probe_after_first_chunk_packets);
    let mut join_to_first_terrain_micros = None;
    let mut play_bundle = PlayPacketBundleAssembler::default();
    let mut interaction_plan = None;
    let mut book_edits_sent = false;
    let mut command_suggestions: Vec<CommandSuggestionObservation> = Vec::new();
    let mut peer: Option<PeerSession> = None;
    progress.note("the login handshake and the first chunk");
    loop {
        let (packet_id, payload) = probe.session.read_packet_or_send_play_tick().await?;
        packets_seen += 1;
        tracing::debug!(
            state = ?probe.state,
            packet_id,
            len = payload.len(),
            "clientbound packet"
        );

        let mut packet_first_chunks = Vec::new();
        match probe.session.decode_packet(packet_id, &payload)? {
            DecodedClientboundPacket::Login(packet) => {
                probe.handle_login_packet(packet).await?;
                packet_first_chunks.push(None);
            }
            DecodedClientboundPacket::Configuration(packet) => {
                probe.handle_configuration_packet(packet).await?;
                packet_first_chunks.push(None);
            }
            DecodedClientboundPacket::Play(packet) => {
                if let Some(batch) = play_bundle.push(packet)? {
                    for packet in batch {
                        probe.apply_observed_play_packet(packet).await?;
                        let first_chunk = probe.first_chunk_summary.as_ref();
                        if join_to_first_terrain_micros.is_none()
                            && first_chunk.is_some_and(|chunk| {
                                chunk.state == bbb_client_world::ChunkState::Decoded
                            })
                        {
                            join_to_first_terrain_micros =
                                Some(connect_started.elapsed().as_micros());
                        }
                        packet_first_chunks.push(first_chunk.map(|chunk| chunk.pos));
                    }
                }
            }
        }

        let should_finish = packet_first_chunks
            .into_iter()
            .any(|packet_first_chunk| drain.observe_packet(packet_first_chunk));
        if options.probe_interactions {
            // The client keeps only the answer it is still waiting on, so each
            // one is recorded as it lands rather than read back at the end.
            record_command_suggestion(&mut probe.world, &mut command_suggestions);
            if interaction_plan.is_none() && interaction_replay_ready(&drain, &probe.chunk_delivery)
            {
                let plan = find_interaction_replay_plan(&probe.world)?;
                send_interaction_replay(
                    &mut probe.session.conn,
                    &mut probe.world,
                    &conn_options.username,
                    conn_options.profile_id,
                    &conn_options.client_information,
                    plan,
                )
                .await?;
                interaction_plan = Some(plan);
            }
            if let Some(plan) = interaction_plan {
                if probe.interaction_replay.is_none() {
                    let report = observe_interaction_replay(
                        &probe.world,
                        plan,
                        &command_suggestions,
                        &probe.wire_positioned_sounds,
                        &probe.block_event_sounds_before,
                        &probe.spawned_entities,
                        &probe.taken_item_entities,
                    );
                    // The session-authority announcements land right after the
                    // login, so requiring them here only fails closed: a server
                    // that never sends them never completes the replay.
                    let authority = observe_session_authority(&probe.world);
                    if !book_edits_sent && report.first_wave_completed() {
                        send_interaction_book_edits(&mut probe.session.conn, &mut probe.world)
                            .await?;
                        book_edits_sent = true;
                    }
                    // The pickup gate is last, so it only ever holds a
                    // replay whose own results have all landed: the drop is out
                    // by then, the session is standing over the cell it landed
                    // in, and nothing the replay has left to send moves it.
                    progress.note(format!(
                        "the interaction replay to settle (completed={}, authority={}, drop pickup={}, report={report:?})",
                        report.completed(),
                        authority.is_some(),
                        probe.drop_pickup_settled(&report),
                    ));
                    if report.completed()
                        && authority.is_some()
                        && probe.drop_pickup_settled(&report)
                    {
                        probe.interaction_replay = Some(report);
                        probe.session_authority = authority;
                        // The single-client replay is done. Everything a player
                        // only ever sees about *another* player needs a second
                        // session, so the peer joins now rather than earlier —
                        // keeping the replay's own results, `/list` included, a
                        // function of one connected player on both servers.
                        peer = Some(PeerSession::spawn(
                            conn_options,
                            options.known_packs_manager.clone(),
                            PeerTarget {
                                entity_id: probe
                                    .world
                                    .local_player_id()
                                    .context("the peer replay needs the primary's entity id")?,
                                position: probe
                                    .world
                                    .local_player_pose()
                                    .context("the peer replay needs the primary's position")?
                                    .position,
                            },
                        )?);
                    }
                }
            }
            if peer.as_ref().is_some_and(PeerSession::ended_unexpectedly) {
                let session = peer.take().expect("the session was just observed");
                session.finish().await?;
                bail!("the peer session ended before the observation completed");
            }
            if let Some(session) = peer.as_mut() {
                match session.observe(&probe.world, &probe.wire_positioned_sounds) {
                    PeerPhase::Joining => {}
                    PeerPhase::Replay(step) => session.request_step(step),
                    PeerPhase::Joined => session.request_stop(),
                    PeerPhase::Left(report) => {
                        probe.peer_observation = Some(report);
                        break;
                    }
                }
                progress.note(session.waiting_for(&probe.world, &probe.wire_positioned_sounds));
            }
        } else if should_finish {
            break;
        }
    }
    let mut extra_serverbound_play_ids = match peer {
        Some(session) => session.finish().await?,
        None => Vec::new(),
    };
    if options.probe_dimension_transitions {
        progress.note("the overworld/nether/end dimension transitions");
        probe.dimension_transitions =
            Some(dimension_transition::run(&mut probe, &mut packets_seen).await?);
    }
    if options.probe_entity_runtime {
        probe.entity_runtime_replay =
            Some(entity_runtime::run(&mut probe, &mut packets_seen, &progress).await?);
    }
    if options.probe_container_menu {
        probe.container_menu =
            Some(container_menu::run(&mut probe, &mut packets_seen, &progress).await?);
    }
    // The walk runs on the primary's own session, after the peer has left so it
    // cannot move the primary out from under an observation the peer is still
    // aiming at, and before the refusal sessions, which are teardown rather
    // than part of this session's replay.
    if options.probe_interactions {
        progress.note("the outward walk out of the spawn column");
        let walk = walk_past_spawn_column(&mut probe).await?;
        probe.outward_walk = Some(walk);
    }
    // The refusals run one at a time, after the peer has gone: each ends the
    // session it is sent on, so none of them can share a connection with
    // anything else the differential reads.
    let refusal_probes = if options.probe_interactions {
        progress.note("the refusal sessions to be turned away");
        let (report, driven) = run_refusal_probes(
            conn_options,
            &options.known_packs_manager,
            &mut probe,
            &mut packets_seen,
        )
        .await?;
        extra_serverbound_play_ids.extend(driven);
        Some(report)
    } else {
        None
    };

    // The operator session runs last of the extra sessions. It is a second
    // player under its own name, so it changes what `/list` answers and what
    // the tab list holds — and every observation that reads either has already
    // been taken by the time it joins. It is also the only session that runs a
    // command with any effect, which is why it is kept away from the replay
    // whose refusals depend on having no permission at all, and why it is
    // asked for separately: a harness that reads the world afterwards would
    // find a level whose game type a command moved.
    if options.probe_operator {
        progress.note("the operator session to run its commands");
        let (report, driven) =
            operator::run_operator_session(conn_options, &options.known_packs_manager).await?;
        extra_serverbound_play_ids.extend(driven);
        probe.operator_session = Some(report);
    }

    // The hold runs after everything else the probe does, so the session that
    // stays connected is one that already completed its whole program.
    progress.note("the requested hold to run out");
    let hold = match options.probe_hold {
        Some(duration) => {
            Some(hold_session(&mut probe, duration, join_to_first_terrain_micros).await?)
        }
        None => None,
    };

    let first_chunk = drain
        .first_chunk()
        .expect("probe exits only after the first chunk is observed");
    Ok(probe.finish(
        packets_seen,
        first_chunk,
        extra_serverbound_play_ids,
        refusal_probes,
        hold,
    ))
}

/// Keeps one finished session connected for a while, still answering whatever
/// the server sends.
///
/// A driver that needs several clients on a server at once cannot get that from
/// clients that leave as soon as they have their first chunk. Holding is not
/// idling: the session keeps decoding and keeps sending its play ticks, so it
/// stays a real player rather than an open socket.
async fn hold_session(
    probe: &mut ProbeContext,
    hold: Duration,
    join_to_first_terrain_micros: Option<u128>,
) -> Result<HoldReport> {
    let deadline = Instant::now() + hold;
    let mut play_bundle = PlayPacketBundleAssembler::default();
    let mut packets = 0usize;
    let mut completed = true;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        let read = timeout(
            remaining,
            probe.session.read_packet_or_eof_or_send_play_tick(),
        )
        .await;
        // Reaching the deadline mid-read is the hold finishing, not a failure.
        let Ok(read) = read else { break };
        let Some((packet_id, payload)) = read? else {
            // The server ended a session that was asked to stay. That is a real
            // result the driver has to see, so it is reported rather than
            // raised: the report says the hold did not complete.
            completed = false;
            break;
        };
        packets += 1;
        match probe.session.decode_packet(packet_id, &payload)? {
            DecodedClientboundPacket::Login(packet) => {
                play_bundle = PlayPacketBundleAssembler::default();
                probe.handle_login_packet(packet).await?;
            }
            DecodedClientboundPacket::Configuration(packet) => {
                play_bundle = PlayPacketBundleAssembler::default();
                probe.handle_configuration_packet(packet).await?;
            }
            DecodedClientboundPacket::Play(packet) => {
                if let Some(batch) = play_bundle.push(packet)? {
                    for packet in batch {
                        probe.handle_play_packet(packet).await?;
                    }
                }
            }
        }
    }
    Ok(HoldReport {
        requested_seconds: hold.as_secs_f64(),
        join_to_first_terrain_micros,
        packets,
        completed,
    })
}

/// Where the peer observation stands from the primary probe's point of view.
enum PeerPhase {
    /// The peer has not become fully visible yet, or its last replay step has
    /// not reached the primary.
    Joining,
    /// The primary is ready for the peer to run one more replay step.
    Replay(PeerReplayStep),
    /// The peer is listed, spawned and done replaying; the primary has
    /// everything it can learn while the peer is connected.
    Joined,
    /// The peer left and the primary saw both halves of the removal.
    Left(PeerObservationReport),
}

/// One step of the peer's movement-state replay.
///
/// Each step changes exactly one shared flag, and the primary only asks for
/// the next one once it has seen the previous change arrive. That is what
/// makes the observation independent of how many of these the server happened
/// to fold into a single `sendDirtyEntityData` flush.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PeerReplayStep {
    StartSprinting,
    ShiftKeyDown,
    StopSprintingAndFallFlying,
    /// `ServerboundInteractPacket` aimed at the primary with the
    /// secondary-action flag clear. `handleInteract` (:1819) writes that flag
    /// into the peer's crouch state before it resolves the target, so the step
    /// settles on the same shared-flags byte every other step is read from —
    /// and it leaves the peer standing for the attack that follows.
    InteractPrimary,
    /// `ServerboundAttackPacket` aimed at the primary. Nothing in this world
    /// can be hurt, so the only thing that reaches the primary is the
    /// no-damage sound `Player.attack` plays instead.
    AttackPrimary,
}

impl PeerReplayStep {
    /// The step to run after this one, in replay order.
    fn next(self) -> Option<Self> {
        match self {
            Self::StartSprinting => Some(Self::ShiftKeyDown),
            Self::ShiftKeyDown => Some(Self::StopSprintingAndFallFlying),
            Self::StopSprintingAndFallFlying => Some(Self::InteractPrimary),
            Self::InteractPrimary => Some(Self::AttackPrimary),
            Self::AttackPrimary => None,
        }
    }

    /// Whether the primary learns this step landed from a sound the server
    /// sent rather than from the peer's shared-flags byte.
    ///
    /// A sound the client made for itself would settle the step whether or
    /// not the peer's swing ever reached the server — the pickup a dropped
    /// item ends in is one that arrives on its own schedule — so the observer
    /// is handed the wire sounds and not every positioned sound the client
    /// produced.
    fn settles_on_a_sound(self) -> bool {
        matches!(self, Self::AttackPrimary)
    }
}

/// A second client session, used to observe what one player sees of another.
///
/// The gate's probe is a single connection, so any mechanism whose only
/// visible result reaches *other* players — the tab-list entry, the spawned
/// avatar, their removal on disconnect — is invisible to it. This session
/// makes that surface part of the differential.
struct PeerSession {
    observer: PeerObserver,
    steps: tokio::sync::mpsc::UnboundedSender<PeerReplayStep>,
    stop: Option<tokio::sync::oneshot::Sender<()>>,
    task: tokio::task::JoinHandle<Result<Vec<i32>>>,
}

/// The observation state machine, split from the session so it can be driven
/// against a world built from packets alone.
struct PeerObserver {
    profile_id: Uuid,
    joined: Option<PeerJoinObservation>,
}

/// What the primary probe saw while the peer was connected.
#[derive(Debug, Clone)]
struct PeerJoinObservation {
    entity_id: i32,
    listed: bool,
    game_mode: String,
    show_hat: bool,
    entity_type_id: i32,
    name: String,
    /// The replay step whose result the primary is currently waiting for, and
    /// the byte it must differ from. `None` once the replay is finished.
    pending_step: Option<PeerReplayStep>,
    last_shared_flags: i8,
    /// How many sounds the primary had heard when the pending step was issued.
    last_sound_count: usize,
    shared_flags: PeerSharedFlagsReport,
    sounds_heard: Vec<SoundObservation>,
}

impl PeerSession {
    fn spawn(
        options: &ConnectionOptions,
        known_packs_manager: Option<Arc<KnownPacksManager>>,
        primary: PeerTarget,
    ) -> Result<Self> {
        let identity = ClientIdentity::offline(PEER_USERNAME);
        let mut peer_options =
            ConnectionOptions::offline_with_identity(&options.address, &identity)
                .context("peer session needs an offline identity")?;
        peer_options.host = options.host.clone();
        peer_options.port = options.port;
        peer_options.timeout = options.timeout;
        peer_options.accepted_code_of_conduct_hash = options.accepted_code_of_conduct_hash;
        peer_options.client_information = options.client_information.clone();
        peer_options.server_resource_pack_policy = options.server_resource_pack_policy;
        let profile_id = peer_options.profile_id;
        let (stop, stop_rx) = tokio::sync::oneshot::channel();
        let (steps, steps_rx) = tokio::sync::mpsc::unbounded_channel();
        let task = tokio::spawn(run_peer_session(
            peer_options,
            known_packs_manager,
            primary,
            stop_rx,
            steps_rx,
        ));
        Ok(Self {
            observer: PeerObserver {
                profile_id,
                joined: None,
            },
            steps,
            stop: Some(stop),
            task,
        })
    }

    fn observe(&mut self, world: &WorldStore, sounds: &[SoundEventState]) -> PeerPhase {
        self.observer.observe(world, sounds)
    }

    /// What the observation would block on if nothing else arrived.
    fn waiting_for(&self, world: &WorldStore, sounds: &[SoundEventState]) -> String {
        self.observer.waiting_for(world, sounds)
    }

    fn request_step(&self, step: PeerReplayStep) {
        // The receiver is gone only if the peer session already ended, which
        // stalls the replay and fails the gate on the probe timeout rather
        // than reporting a value nobody produced.
        let _ = self.steps.send(step);
    }

    /// Whether the peer's session ended before it was asked to.
    ///
    /// The primary waits on what the peer does, so a peer that died takes the
    /// replay with it — reporting that as the peer's own error beats letting
    /// the probe deadline expire on a wait nobody will ever satisfy. A session
    /// that ended after `request_stop` is the normal path, and the primary is
    /// still owed the removal halves of the observation.
    fn ended_unexpectedly(&self) -> bool {
        self.stop.is_some() && self.task.is_finished()
    }

    fn request_stop(&mut self) {
        if let Some(stop) = self.stop.take() {
            // The receiver is gone only if the peer session already ended,
            // which the join below reports.
            let _ = stop.send(());
        }
    }

    /// Stops the peer and hands back the play packets it drove.
    async fn finish(mut self) -> Result<Vec<i32>> {
        self.request_stop();
        self.task.await.context("peer session task panicked")?
    }
}

/// The player the peer replay is aimed at, as the peer needs to know it.
///
/// It travels to the peer once, when its session is created, rather than being
/// read out of the peer's own world: entity tracking is bounded by the
/// server's view distance (`ChunkMap.TrackedEntity.updatePlayer`
/// ChunkMap), and the official server drops a joining player
/// anywhere in a square around the shared spawn (`PlayerSpawnFinder`
/// PlayerSpawnFinder), so the peer cannot count on being able to
/// see the primary when it arrives. The primary does not move between handing
/// this over and the end of the observation, so one copy is enough.
#[derive(Debug, Clone, Copy, PartialEq)]
struct PeerTarget {
    entity_id: i32,
    position: packets::Vec3d,
}

impl PeerObserver {
    /// Advances the observation using only what the primary probe's own world
    /// has applied, so the report is what a real client could see.
    fn observe(&mut self, world: &WorldStore, sounds: &[SoundEventState]) -> PeerPhase {
        let Some(joined) = self.joined.clone() else {
            let Some(entry) = world.player_info_entry(self.profile_id) else {
                return PeerPhase::Joining;
            };
            // `ClientPacketListener` drops a spawn whose player-info entry has
            // not arrived, so the entity is the later of the two.
            let Some(transform) = world
                .entity_transforms()
                .into_iter()
                .find(|transform| transform.uuid == self.profile_id)
            else {
                return PeerPhase::Joining;
            };
            // The spawn bundle is the baseline every later flag change is read
            // against, so it is sampled from the same packet batch that made
            // the avatar visible.
            let at_join = world
                .entity_shared_flags(transform.id)
                .expect("the avatar was just found in the same world");
            self.joined = Some(PeerJoinObservation {
                entity_id: transform.id,
                listed: entry.listed,
                game_mode: entry.game_mode.clone(),
                show_hat: entry.show_hat,
                entity_type_id: transform.entity_type_id,
                name: entry.profile.name.clone(),
                pending_step: Some(PeerReplayStep::StartSprinting),
                last_shared_flags: at_join,
                last_sound_count: sounds.len(),
                shared_flags: PeerSharedFlagsReport {
                    at_join,
                    after_start_sprinting: at_join,
                    after_shift_key_down: at_join,
                    after_stop_sprinting_and_fall_flying: at_join,
                    after_interact: at_join,
                },
                sounds_heard: Vec::new(),
            });
            return PeerPhase::Replay(PeerReplayStep::StartSprinting);
        };
        if let Some(step) = joined.pending_step {
            let observed = world
                .entity_shared_flags(joined.entity_id)
                .unwrap_or(joined.last_shared_flags);
            let settled = if step.settles_on_a_sound() {
                sounds.len() > joined.last_sound_count
            } else {
                observed != joined.last_shared_flags
            };
            if !settled {
                return PeerPhase::Joining;
            }
            let next = step.next();
            let here = world
                .local_player_pose()
                .map(|pose| [pose.position.x, pose.position.y, pose.position.z])
                .unwrap_or_default();
            let fresh = sounds[joined.last_sound_count.min(sounds.len())..]
                .iter()
                .map(|sound| sound_observation(sound, here))
                .collect::<Vec<_>>();
            let joined = self
                .joined
                .as_mut()
                .expect("the observation was cloned from Some");
            joined.last_shared_flags = observed;
            joined.last_sound_count = sounds.len();
            joined.pending_step = next;
            joined.sounds_heard.extend(fresh);
            match step {
                PeerReplayStep::StartSprinting => {
                    joined.shared_flags.after_start_sprinting = observed;
                }
                PeerReplayStep::ShiftKeyDown => {
                    joined.shared_flags.after_shift_key_down = observed;
                }
                PeerReplayStep::StopSprintingAndFallFlying => {
                    joined.shared_flags.after_stop_sprinting_and_fall_flying = observed;
                }
                PeerReplayStep::InteractPrimary => {
                    joined.shared_flags.after_interact = observed;
                }
                // The attack settles on the sound the observer just heard,
                // which the sound list above already carries.
                PeerReplayStep::AttackPrimary => {}
            }
            return match next {
                Some(step) => PeerPhase::Replay(step),
                None => PeerPhase::Joined,
            };
        }
        let entry_removed = world.player_info_entry(self.profile_id).is_none();
        let entity_removed = !world
            .entity_transforms()
            .into_iter()
            .any(|transform| transform.id == joined.entity_id);
        if entry_removed && entity_removed {
            PeerPhase::Left(PeerObservationReport {
                name: joined.name,
                listed: joined.listed,
                game_mode: joined.game_mode,
                show_hat: joined.show_hat,
                entity_type_id: joined.entity_type_id,
                removed_from_player_list: true,
                entity_removed: true,
                shared_flags: joined.shared_flags,
                sounds_heard: joined.sounds_heard,
            })
        } else {
            PeerPhase::Joined
        }
    }

    /// Describes what this observation is still blocked on, in the terms the
    /// world it reads can answer.
    ///
    /// The probe's only failure mode for a step that never lands is the
    /// deadline, and a bare deadline cannot tell a peer the primary never saw
    /// from a peer whose swing never came back as a sound. This says which.
    fn waiting_for(&self, world: &WorldStore, sounds: &[SoundEventState]) -> String {
        let listed = world.player_info_entry(self.profile_id).is_some();
        let avatar = world
            .entity_transforms()
            .into_iter()
            .find(|transform| transform.uuid == self.profile_id);
        let Some(joined) = self.joined.as_ref() else {
            return format!(
                "the peer to become visible (player-list entry {}, avatar {})",
                if listed { "seen" } else { "missing" },
                if avatar.is_some() { "seen" } else { "missing" },
            );
        };
        let here = world.local_player_pose().map(|pose| pose.position);
        let there = world
            .entity_transforms()
            .into_iter()
            .find(|transform| transform.id == joined.entity_id)
            .map(|transform| transform.position);
        let apart = match (here, there) {
            (Some(here), Some(there)) => {
                let dx = here.x - there.x;
                let dy = here.y - there.y;
                let dz = here.z - there.z;
                format!("{:.2}", (dx * dx + dy * dy + dz * dz).sqrt())
            }
            _ => "unknown".to_owned(),
        };
        let Some(step) = joined.pending_step else {
            return format!(
                "the peer's removal (player-list entry {}, avatar {})",
                if listed { "still listed" } else { "gone" },
                if there.is_some() {
                    "still shown"
                } else {
                    "gone"
                },
            );
        };
        let observed = world.entity_shared_flags(joined.entity_id);
        format!(
            "peer replay step {step:?} to settle (entity {}, {apart} blocks away, \
             shared flags {observed:?} against {}, {} sounds heard against {})",
            joined.entity_id,
            joined.last_shared_flags,
            sounds.len(),
            joined.last_sound_count,
        )
    }
}

/// Walks this session's player onto a position, one accepted hop per tick.
///
/// `handleMovePlayer` rejects a
/// hop whose squared distance from the position the tick started at exceeds
/// 100, so the walk is split into hops under ten blocks and paced a tick
/// apart. The endpoint of the last hop is the target itself; `keep_clear` is
/// how wide a berth every hop before it gives that target, which is what
/// [`WALK_TARGET_STANDOFF`] exists to explain. `await_chunk_centres` makes a hop
/// that crosses a chunk wait for the server's cache-centre acknowledgement, so
/// the outward generator probe cannot queue several moves behind fresh chunks.
async fn walk_to(
    probe: &mut ProbeContext,
    target: packets::Vec3d,
    keep_clear: f64,
    on_ground: bool,
    await_chunk_centres: bool,
) -> Result<()> {
    let mut pose = probe
        .world
        .local_player_pose()
        .context("walking needs the session's own pose")?;
    let mut position = pose.position;
    while let Some(hop) = walk_hop(position, target, keep_clear) {
        let previous_chunk = column_of(position);
        position = hop.position;
        let (id, payload) = packets::encode_play_move_player_pos_rot(
            position.x, position.y, position.z, pose.y_rot, pose.x_rot, on_ground, false,
        );
        probe.conn.send_packet(id, &payload).await?;
        pose.position = position;
        pose.on_ground = on_ground;
        probe.world.set_local_player_pose(pose);
        let entered_chunk = column_of(position);
        if await_chunk_centres && entered_chunk != previous_chunk {
            await_walk_arrival(probe, entered_chunk).await?;
            pose = probe
                .world
                .local_player_pose()
                .context("walking lost the session's own pose while awaiting a chunk")?;
            position = pose.position;
        }
        if hop.arrives {
            return Ok(());
        }
        tokio::time::sleep(WALK_HOP_INTERVAL).await;
    }
    Ok(())
}

/// One hop of a walk: where it claims to be, and whether that is the target.
#[derive(Debug, Clone, Copy, PartialEq)]
struct WalkHop {
    position: packets::Vec3d,
    arrives: bool,
}

/// Plans the next hop of a walk from `from` to `target`, or `None` once the
/// walk is already standing on it.
///
/// Split out from [`walk_to`] so the two properties the walk depends on — that
/// no hop is longer than one tick's movement allowance, and that no hop before
/// the last one lands within `keep_clear` of the target — can be checked
/// without a server to walk on.
fn walk_hop(from: packets::Vec3d, target: packets::Vec3d, keep_clear: f64) -> Option<WalkHop> {
    let delta = [target.x - from.x, target.y - from.y, target.z - from.z];
    let distance = (delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]).sqrt();
    if distance <= f64::EPSILON {
        return None;
    }
    if distance <= WALK_HOP_BLOCKS {
        // Landing on the target exactly, rather than a rounding error away from
        // it, is what makes the last hop's box the one the target is already
        // standing in.
        return Some(WalkHop {
            position: target,
            arrives: true,
        });
    }
    // Stopping short of `keep_clear` rather than a full hop's worth keeps the
    // walk converging: the distance left after this hop is the larger of
    // `keep_clear` and one hop less than it was.
    let step = WALK_HOP_BLOCKS.min(distance - keep_clear) / distance;
    Some(WalkHop {
        position: packets::Vec3d {
            x: from.x + delta[0] * step,
            y: from.y + delta[1] * step,
            z: from.z + delta[2] * step,
        },
        arrives: false,
    })
}

/// `InventoryMenu` slot 36, the first hotbar column (InventoryMenu.java:15-27).
const FIRST_HOTBAR_MENU_SLOT: i16 = 36;

/// Leaves room under the squared-distance limit for server-side vertical drift.
const WALK_HOP_BLOCKS: f64 = 4.0;
/// How wide a berth the peer's approach gives the primary until its last hop.
///
/// The last hop lands exactly where the primary stands, which cannot collide
/// with a block because the primary is standing in it. Every hop before it
/// lands somewhere nobody has vouched for, and `handleMovePlayer` silently
/// rejects a hop whose box collides with anything the old box did not
/// (ServerGamePacketListenerImpl, deciding on
/// `isEntityCollidingWithAnythingNew`). A rejected hop is answered
/// with a teleport back that `updateAwaitingTeleport`
/// then makes swallow every later hop, so the peer stops short with nothing
/// reported and the attack step — the one thing vanilla gates on range
/// (`handleAttack`) — never produces the sound the observation settles
/// on. The interaction replay leaves a placed block with a note block on it
/// one cell from where it stands and a carpet under its own feet, so three
/// blocks clears what it built. Three and a half preserves that margin while
/// still fitting inside the final four-block hop.
const WALK_TARGET_STANDOFF: f64 = 3.5;
/// Longer than the 50ms tick that resets the allowance.
const WALK_HOP_INTERVAL: std::time::Duration = std::time::Duration::from_millis(120);

/// The column a position stands in.
///
/// `ChunkPos.containing` goes through
/// `SectionPos.blockToSectionCoord`, which is an
/// arithmetic shift — floor division, not the truncating one Rust's `/` gives.
/// The two disagree either side of the origin, and the official server spreads
/// new players over a square around spawn (`PlayerSpawnFinder`) that straddles
/// it, so the difference is reachable rather than theoretical.
fn column_of(position: packets::Vec3d) -> ChunkPos {
    ChunkPos {
        x: (position.x.floor() as i32).div_euclid(16),
        z: (position.z.floor() as i32).div_euclid(16),
    }
}

/// Walks the session out of the column it spawned in and reads what it found.
///
/// The spawn column is the only one `first_chunk_*` ever describes, so on its
/// own it cannot tell a server whose world is one column from a server whose
/// generator runs everywhere. This walks [`OUTWARD_WALK_CHUNKS`] columns along
/// +X — a column both servers already hold, see that constant — waits for the
/// server to say the session is standing there, and probes the terrain the way
/// the spawn column is probed.
async fn walk_past_spawn_column(probe: &mut ProbeContext) -> Result<OutwardWalkReport> {
    let pose = probe
        .world
        .local_player_pose()
        .context("the outward walk needs the session's own pose")?;
    let start_chunk = column_of(pose.position);
    let walked_chunk = ChunkPos {
        x: start_chunk.x + OUTWARD_WALK_CHUNKS,
        z: start_chunk.z,
    };
    let center = column_center_block(walked_chunk);
    let target_x = f64::from(center.x) + 0.5;
    let target_z = f64::from(center.z) + 0.5;
    let route_y = outward_walk_clearance_y(&probe.world, pose.position, target_x, target_z)?;
    if route_y > pose.position.y {
        walk_to(
            probe,
            packets::Vec3d {
                y: route_y,
                ..pose.position
            },
            0.0,
            false,
            false,
        )
        .await?;
        tokio::time::sleep(WALK_HOP_INTERVAL).await;
    }
    walk_to(
        probe,
        packets::Vec3d {
            x: target_x,
            y: route_y,
            z: target_z,
        },
        0.0,
        false,
        true,
    )
    .await?;
    await_walk_arrival(probe, walked_chunk).await?;
    let walked_chunk_summary = probe
        .world
        .probe_chunk(walked_chunk)
        .map(ChunkProbeSummary::from_column)
        .expect("the walk only returns once the column has arrived");
    let walked_chunk_center_block = probe.world.probe_block(center).with_context(|| {
        format!("the column at {walked_chunk:?} arrived with no section at y {COLUMN_PROBE_Y}")
    })?;
    Ok(OutwardWalkReport {
        start_chunk,
        walked_chunk,
        walked_chunk_center_block,
        walked_chunk_summary,
    })
}

fn outward_walk_clearance_y(
    world: &WorldStore,
    from: packets::Vec3d,
    target_x: f64,
    target_z: f64,
) -> Result<f64> {
    let min_x = (from.x.min(target_x).floor() as i32).saturating_sub(1);
    let max_x = (from.x.max(target_x).floor() as i32).saturating_add(1);
    let min_z = (from.z.min(target_z).floor() as i32).saturating_sub(1);
    let max_z = (from.z.max(target_z).floor() as i32).saturating_add(1);
    let dimension = world.dimension();
    let max_y = dimension.min_y.saturating_add(dimension.height);
    let mut top = None;
    for x in min_x..=max_x {
        for z in min_z..=max_z {
            for y in (dimension.min_y..max_y).rev() {
                let block = world.probe_block(BlockPos { x, y, z }).with_context(|| {
                    format!("the outward walk corridor has no loaded block at [{x},{y},{z}]")
                })?;
                if block.block_name.as_deref().is_some_and(block_name_is_air) {
                    continue;
                }
                top = Some(top.map_or(y, |current: i32| current.max(y)));
                break;
            }
        }
    }
    let clearance = top
        .and_then(|y| y.checked_add(2))
        .map(f64::from)
        .unwrap_or(from.y)
        .max(from.y);
    if clearance + 2.0 > f64::from(max_y) {
        bail!("the outward walk corridor has no player-height clearance below y {max_y}");
    }
    Ok(clearance)
}

/// How long the walk waits to land.
///
/// The last hop reaches `getChunkSource().move`
/// (ServerGamePacketListenerImpl.java:1157) inside the handler. BBB's serial
/// world route may still be finishing the departing peer's fresh Noise FULL
/// batch before it handles that hop, so this wait must cover that bounded
/// predecessor as well as the walk's own chunk batch. Running out of time is
/// still an error rather than an empty field.
const OUTWARD_WALK_ARRIVAL_TIMEOUT: Duration = Duration::from_secs(120);

/// Pumps the session until the walk has landed: the server has re-centred this
/// session's chunk cache on the column it walked into, and that column has
/// arrived.
///
/// Both halves are the point. The centre is the server's own acknowledgement
/// that the walk was accepted — `ChunkMap.move`
/// reaches `applyChunkTrackingView`, which sends
/// `ClientboundSetChunkCacheCenterPacket` whenever the tracked centre moved —
/// and it names the player's own column, so unlike a chunk count it carries no
/// dependence on either server's view distance. Waiting only on the column
/// would let a fire-and-forget walk pass while the column being probed was one
/// the login had already delivered.
async fn await_walk_arrival(probe: &mut ProbeContext, pos: ChunkPos) -> Result<()> {
    let deadline = Instant::now() + OUTWARD_WALK_ARRIVAL_TIMEOUT;
    let mut play_bundle = PlayPacketBundleAssembler::default();
    while probe.world.chunk_cache_center() != Some(pos) || probe.world.probe_chunk(pos).is_none() {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            bail!(
                "the walk into {pos:?} never landed: chunk cache centre {:?}, column {}",
                probe.world.chunk_cache_center(),
                match probe.world.probe_chunk(pos) {
                    Some(_) => "delivered",
                    None => "never sent",
                }
            );
        }
        let read = timeout(
            remaining,
            probe.session.read_packet_or_eof_or_send_play_tick(),
        )
        .await;
        // Reaching the deadline mid-read is the deadline above, which the next
        // pass reports rather than this one guessing at.
        let Ok(read) = read else { continue };
        let Some((packet_id, payload)) = read? else {
            bail!("the session ended before the walk into {pos:?} landed");
        };
        if probe.state != ConnectionState::Play {
            continue;
        }
        let packet = probe.session.decode_play_packet(packet_id, &payload)?;
        if let Some(batch) = play_bundle.push(packet)? {
            for packet in batch {
                probe.handle_play_packet(packet).await?;
            }
        }
    }
    Ok(())
}

/// A received sound as the differential compares it.
///
/// `ClientboundSoundPacket` quantises its position to eighths of a block
/// (`ClientboundSoundPacket.java:15, 34-40`), so "came from here" is only
/// decidable to that resolution — which is exactly the resolution vanilla
/// itself keeps.
fn sound_observation(sound: &SoundEventState, here: [f64; 3]) -> SoundObservation {
    SoundObservation {
        sound: sound_resource_id(sound),
        source: sound.source.clone(),
        volume: sound.volume,
        pitch: sound.pitch,
        at_local_player_position: sound_emitted_at(sound, here),
    }
}

/// The same sound, positioned against the note block instead of the listener.
///
/// The note is emitted at the block (`NoteBlock.triggerEvent`,
/// NoteBlock), so the reference point is the only thing that
/// differs between this and [`sound_observation`].
fn note_block_sound_observation(
    sound: &SoundEventState,
    note_block: [f64; 3],
) -> SoundAtBlockObservation {
    SoundAtBlockObservation {
        sound: sound_resource_id(sound),
        source: sound.source.clone(),
        volume: sound.volume,
        pitch: sound.pitch,
        at_note_block: sound_emitted_at(sound, note_block),
    }
}

/// The sound's registry resource id, resolved through the built-in catalog. An
/// id outside it is reported rather than dropped, so a server naming a sound
/// the other does not still differs.
fn sound_resource_id(sound: &SoundEventState) -> String {
    sound
        .sound
        .registry_id
        .and_then(vanilla_sound_event_resource_id)
        .map(str::to_string)
        .unwrap_or_else(|| format!("{:?}", sound.sound))
}

/// Whether the packet says the sound came out of `here`.
///
/// One quantum of slack and no more: the only approximation between the two is
/// the packet's own truncation. Comparing against another entity's position
/// instead would have to allow for the tracker's delta encoding and its
/// client-side interpolation as well.
fn sound_emitted_at(sound: &SoundEventState, here: [f64; 3]) -> bool {
    [sound.position.x, sound.position.y, sound.position.z]
        .iter()
        .zip(here)
        .all(|(emitted, here)| (emitted - here).abs() < SOUND_POSITION_QUANTUM)
}

/// The block distance one unit of `ClientboundSoundPacket.LOCATION_ACCURACY`
/// covers (`ClientboundSoundPacket.java:15`).
const SOUND_POSITION_QUANTUM: f64 = 1.0 / 8.0;

/// The peer's username. Vanilla derives an offline UUID from it, so both
/// servers give the peer the same identity.
const PEER_USERNAME: &str = "server-peer";

/// Runs one replay step on the peer's own connection.
///
/// `ServerboundPlayerCommandPacket` carries an entity id the vanilla handler
/// never reads — it acts on the connection's own player — but the peer sends
/// its real id so a server that did read it still gets the right one.
async fn send_peer_replay_step(
    probe: &mut ProbeContext,
    step: PeerReplayStep,
    primary: PeerTarget,
) -> Result<()> {
    let entity_id = probe
        .world
        .local_player_id()
        .context("peer replay needs the login's player id")?;
    let primary_entity_id = primary.entity_id;
    let command = |action| {
        packets::encode_play_player_command(packets::PlayerCommand {
            entity_id,
            action,
            data: 0,
        })
    };
    let encoded = match step {
        PeerReplayStep::StartSprinting => {
            vec![command(PlayerCommandAction::StartSprinting)]
        }
        PeerReplayStep::ShiftKeyDown => {
            vec![packets::encode_play_player_input(packets::PlayerInput {
                shift: true,
                ..packets::PlayerInput::default()
            })]
        }
        // Both land in one step because only the first of them changes the
        // byte: `stopFallFlying` re-sends the value it just wrote back. Pairing
        // them keeps the observation a single settled change while still
        // driving the fall-flying path.
        PeerReplayStep::StopSprintingAndFallFlying => vec![
            command(PlayerCommandAction::StopSprinting),
            command(PlayerCommandAction::StartFallFlying),
        ],
        // Both handlers gate on the peer's hand reaching the primary, and the
        // official server spreads new players over a square around the shared
        // spawn (`PlayerSpawnFinder`), so how far
        // apart they start is that server's own business. Reaching the primary
        // first is what makes the two aimed-at steps mean the same thing on
        // both servers.
        //
        // The walk belongs to this step and not an earlier one because the
        // peer is crouching by the time it runs: `ShiftKeyDown` set the flag
        // two steps ago and only the interact below clears it.
        // `Player.getMovementEmission` answers `NONE`
        // for a player who is on the ground and crouching, and `Entity.move`
        // asks for movement effects only when the emission emits anything
        // — so a crouched walk broadcasts no step
        // sounds. That matters because [`PeerReplayStep::AttackPrimary`]
        // settles on the next sound the primary hears, and a step sound would
        // settle it before the attack ever reached the server.
        PeerReplayStep::InteractPrimary => {
            send_encoded(
                &mut probe.conn,
                packets::encode_play_player_abilities(packets::PlayerAbilitiesCommand {
                    flying: true,
                }),
            )
            .await?;
            let pose = probe
                .world
                .local_player_pose()
                .context("the peer approach needs its own pose")?;
            let route_y = outward_walk_clearance_y(
                &probe.world,
                pose.position,
                primary.position.x,
                primary.position.z,
            )?;
            if route_y > pose.position.y {
                walk_to(
                    probe,
                    packets::Vec3d {
                        y: route_y,
                        ..pose.position
                    },
                    0.0,
                    false,
                    false,
                )
                .await?;
            }
            walk_to(
                probe,
                packets::Vec3d {
                    x: primary.position.x,
                    y: route_y,
                    z: primary.position.z,
                },
                0.0,
                false,
                false,
            )
            .await?;
            walk_to(probe, primary.position, WALK_TARGET_STANDOFF, true, false).await?;
            vec![packets::encode_play_interact_entity(
                packets::InteractEntity {
                    entity_id: primary_entity_id,
                    hand: packets::InteractionHand::MainHand,
                    location: packets::Vec3d::default(),
                    using_secondary_action: false,
                },
            )]
        }
        PeerReplayStep::AttackPrimary => {
            vec![packets::encode_play_attack_entity(packets::AttackEntity {
                entity_id: primary_entity_id,
            })]
        }
    };
    for (id, payload) in encoded {
        probe.conn.send_packet(id, &payload).await?;
    }
    Ok(())
}

/// Drives a second client to Play and keeps it there until asked to stop.
///
/// It reuses the probe's own packet handling so the peer answers keep-alives
/// and completes the spawn handshake exactly like the primary session.
async fn run_peer_session(
    options: ConnectionOptions,
    known_packs_manager: Option<Arc<KnownPacksManager>>,
    primary: PeerTarget,
    mut stop: tokio::sync::oneshot::Receiver<()>,
    mut steps: tokio::sync::mpsc::UnboundedReceiver<PeerReplayStep>,
) -> Result<Vec<i32>> {
    let mut probe = ProbeContext::new(RawConnection::connect(&options.address, None).await?);
    probe.known_packs_manager = known_packs_manager;
    probe.accepted_code_of_conduct_hash = options.accepted_code_of_conduct_hash;
    probe.client_information = options.client_information.clone();

    probe
        .session
        .start_login(
            &options.host,
            options.port,
            ClientIntent::Login,
            &options.username,
            options.profile_id,
        )
        .await?;

    let mut play_bundle = PlayPacketBundleAssembler::default();
    let mut pending_steps: std::collections::VecDeque<PeerReplayStep> =
        std::collections::VecDeque::new();
    loop {
        // A step runs only once the session has declared itself loaded:
        // `handlePlayerCommand`, `handlePlayerInput` and `handleMovePlayer`
        // are all gated on `hasClientLoaded`
        // so a step sent any earlier
        // would be dropped and the primary would wait on a change that never
        // comes.
        while probe.player_loaded_sent {
            let Some(&step) = pending_steps.front() else {
                break;
            };
            pending_steps.pop_front();
            send_peer_replay_step(&mut probe, step, primary).await?;
        }
        let (packet_id, payload) = tokio::select! {
            _ = &mut stop => return Ok(probe.conn.serverbound_play_ids()),
            step = steps.recv() => {
                match step {
                    Some(step) => {
                        pending_steps.push_back(step);
                        continue;
                    }
                    // The primary dropped the sender, which only happens when
                    // it is finished with this session.
                    None => return Ok(probe.conn.serverbound_play_ids()),
                }
            }
            packet = probe.session.read_packet_or_send_play_tick() => packet?,
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
                        probe.handle_play_packet(packet).await?;
                    }
                }
            }
        }
    }
}

/// How long a refusal session waits for the close it asked for.
///
/// `disconnect` runs inside the handler on both servers, so the answer is one
/// tick away; this is generous enough that only a server which did not refuse
/// at all reaches it.
const REFUSAL_CLOSE_TIMEOUT: Duration = Duration::from_secs(5);

/// One request that ends the session it is sent on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RefusalCase {
    CookieResponse,
    UnsolicitedKeepAlive,
    ConfigurationAcknowledged,
    MalformedPacketBody,
    MalformedComponentNbt,
}

impl RefusalCase {
    /// Each case needs its own session, and both servers refuse a second login
    /// under a name already connected, so each needs its own name too.
    fn username(self) -> &'static str {
        match self {
            Self::CookieResponse => "refuse-cookie",
            Self::UnsolicitedKeepAlive => "refuse-alive",
            Self::ConfigurationAcknowledged => "refuse-config",
            Self::MalformedPacketBody => "refuse-body",
            Self::MalformedComponentNbt => "refuse-nbt",
        }
    }

    fn packet(self) -> Result<(i32, Vec<u8>)> {
        Ok(match self {
            Self::CookieResponse => packets::encode_play_cookie_response(REFUSED_COOKIE_KEY, None)?,
            Self::UnsolicitedKeepAlive => packets::encode_play_keep_alive(REFUSED_KEEP_ALIVE_ID),
            Self::ConfigurationAcknowledged => packets::encode_play_configuration_acknowledged(),
            // Both malformed cases start from what the encoder produces and
            // damage it, so the packet id and every byte up to the damage are
            // the real ones. Writing the bytes by hand would be a second
            // encoder, and one that could drift into sending something no
            // server was ever asked to read.
            Self::MalformedPacketBody => {
                let (id, payload) = malformed_body_source()?;
                // Half a body: the fields that remain are a prefix of a real
                // packet, and the ones that do not are what the decoder runs
                // out of bytes for.
                (id, payload[..payload.len() / 2].to_vec())
            }
            Self::MalformedComponentNbt => {
                let (id, mut payload) = malformed_nbt_source()?;
                // The component value is length-prefixed and its NBT is the
                // tail, so the last byte is the compound's terminating TAG_End
                // (`encodes_custom_data_patch_with_delimited_compound_tag`).
                // Replacing it leaves every length intact and leaves the NBT
                // parser reading a tag type that does not exist.
                let last = payload
                    .last_mut()
                    .ok_or_else(|| anyhow!("the component packet encoded to nothing"))?;
                *last = MALFORMED_NBT_TAG_TYPE;
                (id, payload)
            }
        })
    }
}

/// A tag type NBT has never had, written where a valid one is required.
///
/// `NbtType` covers 0..=12 (`TagTypes.java`), so this is refused by any parser
/// that reads a tag type at all, rather than being a valid tag whose payload
/// happens to be wrong.
const MALFORMED_NBT_TAG_TYPE: u8 = 0x7F;

/// The real packet the malformed-body case damages: a chat message, whose body
/// is a string, two longs, an optional signature and the last-seen update, so
/// half of it is a body no reader can finish.
fn malformed_body_source() -> Result<(i32, Vec<u8>)> {
    Ok(packets::encode_play_chat_message(
        &packets::ChatMessage::unsigned(
            MALFORMED_BODY_MESSAGE,
            MALFORMED_BODY_TIMESTAMP_MILLIS,
            MALFORMED_BODY_SALT,
        ),
    ))
}

/// The real packet the malformed-NBT case damages: a creative slot write
/// carrying a `custom_data` component, which is the one component that is
/// literally an NBT compound on the wire.
///
/// It has to be a creative write on a creative session, because vanilla skips
/// this packet outright when the sender is not in creative
/// (`GameProtocols.HAS_INFINITE_MATERIALS` :43-47) and would never reach the
/// NBT at all.
fn malformed_nbt_source() -> Result<(i32, Vec<u8>)> {
    Ok(packets::encode_play_set_creative_mode_slot(
        &SetCreativeModeSlot {
            slot_num: MALFORMED_NBT_SLOT,
            item: ItemStackSummary {
                item_id: Some(VANILLA_ITEM_STONE_ID),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    added_type_ids: vec![VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID],
                    custom_data: Some(NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                        name: MALFORMED_NBT_ENTRY_NAME.into(),
                        value: NbtSummaryValue::Int(1),
                    }])),
                    ..DataComponentPatchSummary::default()
                }
                .into(),
            },
        },
    )?)
}

/// Runs every refusal session in turn and reports what each server did.
///
/// They run one at a time and after the peer has left: a refusal session is a
/// joined player like any other, and running them alongside the replay would
/// make the `/list` reply and the tab-list observations depend on how far
/// along they happened to be.
async fn run_refusal_probes(
    options: &ConnectionOptions,
    known_packs_manager: &Option<Arc<KnownPacksManager>>,
    primary: &mut ProbeContext,
    packets_seen: &mut usize,
) -> Result<(RefusalProbeReport, Vec<i32>)> {
    let mut driven = Vec::new();
    let (cookie_response, cookie_ids) = run_refusal_session(
        options,
        known_packs_manager,
        RefusalCase::CookieResponse,
        primary,
        packets_seen,
    )
    .await?;
    driven.extend(cookie_ids);
    let (unsolicited_keep_alive, keep_alive_ids) = run_refusal_session(
        options,
        known_packs_manager,
        RefusalCase::UnsolicitedKeepAlive,
        primary,
        packets_seen,
    )
    .await?;
    driven.extend(keep_alive_ids);
    let (configuration_acknowledged, configuration_ids) = run_refusal_session(
        options,
        known_packs_manager,
        RefusalCase::ConfigurationAcknowledged,
        primary,
        packets_seen,
    )
    .await?;
    driven.extend(configuration_ids);
    let (malformed_packet_body, body_ids) = run_refusal_session(
        options,
        known_packs_manager,
        RefusalCase::MalformedPacketBody,
        primary,
        packets_seen,
    )
    .await?;
    driven.extend(body_ids);
    let (malformed_component_nbt, nbt_ids) = run_refusal_session(
        options,
        known_packs_manager,
        RefusalCase::MalformedComponentNbt,
        primary,
        packets_seen,
    )
    .await?;
    driven.extend(nbt_ids);
    let report = RefusalProbeReport {
        cookie_response,
        unsolicited_keep_alive,
        configuration_acknowledged,
        malformed_packet_body,
        malformed_component_nbt,
    };
    // Every one of these must have joined and then been thrown out. A server
    // that kept one of these sessions alive is a per-side failure rather than
    // a difference to report, and saying so here names which one.
    if !report.completed() {
        bail!("a refusal session was not refused: {report:?}");
    }
    Ok((report, driven))
}

/// Whether an error is the connection having been ended by the other side.
///
/// A graceful close arrives as end of file and is read as such; a close that
/// discards unread bytes arrives as a reset, and on the sending side as a
/// broken pipe. All three say the same thing about the session.
fn connection_ended(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        cause.downcast_ref::<std::io::Error>().is_some_and(|error| {
            matches!(
                error.kind(),
                std::io::ErrorKind::ConnectionReset
                    | std::io::ErrorKind::ConnectionAborted
                    | std::io::ErrorKind::BrokenPipe
                    | std::io::ErrorKind::UnexpectedEof
            )
        })
    })
}

/// Joins, sends the one packet, and reports how the server ended the session.
async fn run_refusal_session(
    options: &ConnectionOptions,
    known_packs_manager: &Option<Arc<KnownPacksManager>>,
    case: RefusalCase,
    primary: &mut ProbeContext,
    packets_seen: &mut usize,
) -> Result<(RefusalObservation, Vec<i32>)> {
    let identity = ClientIdentity::offline(case.username());
    let mut session_options = ConnectionOptions::offline_with_identity(&options.address, &identity)
        .context("refusal session needs an offline identity")?;
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

    let mut play_bundle = PlayPacketBundleAssembler::default();
    let mut observation = RefusalObservation {
        reached_play: false,
        session_ended: false,
        disconnect_reason: None,
    };
    'session: loop {
        // The packet goes out only once the session is a joined, loaded one,
        // so what it proves is about the handler rather than about a
        // half-finished login.
        if !observation.reached_play && probe.player_loaded_sent {
            let (id, payload) = case.packet()?;
            probe.conn.send_packet(id, &payload).await?;
            observation.reached_play = true;
        }
        // Past the trigger the only thing left to wait for is the close, and
        // both servers make it within a tick. Bounding it here reports a
        // server that kept the session as itself rather than as the whole
        // probe running out of time.
        let read = if observation.reached_play {
            match timeout(
                REFUSAL_CLOSE_TIMEOUT,
                probe.session.read_packet_or_eof_or_send_play_tick(),
            )
            .await
            {
                Ok(Ok(result)) => result,
                // A server that refuses a packet in its decoder still has the
                // rest of that frame sitting unread, so closing on it resets
                // the connection rather than ending it politely. The reset is
                // the refusal; only a session that survives it is a failure.
                Ok(Err(error)) if connection_ended(&error) => {
                    observation.session_ended = true;
                    break;
                }
                Ok(Err(error)) => return Err(error),
                Err(_) => break,
            }
        } else {
            probe.session.read_packet_or_eof_or_send_play_tick().await?
        };
        let Some((packet_id, payload)) = read else {
            observation.session_ended = true;
            break;
        };
        match probe.session.decode_packet(packet_id, &payload)? {
            DecodedClientboundPacket::Login(packet) => {
                probe.handle_login_packet(packet).await?;
            }
            DecodedClientboundPacket::Configuration(packet) => {
                probe.handle_configuration_packet(packet).await?;
            }
            DecodedClientboundPacket::Play(packet) => {
                // The refusal is the result here rather than a session
                // failure, so it is read out and the close waited for instead
                // of being handed to the play handler, which treats a
                // disconnect as the end of a session that should have lived.
                if let packets::PlayClientbound::Disconnect(disconnect) = &packet {
                    observation.disconnect_reason = Some(disconnect.reason.clone());
                    continue;
                }
                if let Some(batch) = play_bundle.push(packet)? {
                    for packet in batch {
                        if let Err(error) = probe.handle_play_packet(packet).await {
                            // Buffered Play packets can need replies after the refusal closed us.
                            if observation.reached_play && connection_ended(&error) {
                                observation.session_ended = true;
                                break 'session;
                            }
                            return Err(error);
                        }
                    }
                }
            }
        }
    }
    let driven = probe.conn.serverbound_play_ids();
    if observation.reached_play && observation.session_ended {
        await_refused_player_removal(primary, identity.profile_id, packets_seen).await?;
    }
    Ok((observation, driven))
}

async fn await_refused_player_removal(
    primary: &mut ProbeContext,
    profile_id: Uuid,
    packets_seen: &mut usize,
) -> Result<()> {
    // A closed socket can precede the world tick that releases its player slot.
    timeout(REFUSAL_CLOSE_TIMEOUT, async {
        let mut play_bundle = PlayPacketBundleAssembler::default();
        loop {
            let (packet_id, payload) = primary.session.read_packet_or_send_play_tick().await?;
            *packets_seen += 1;
            let packet = primary.session.decode_play_packet(packet_id, &payload)?;
            if let Some(batch) = play_bundle.push(packet)? {
                let removed = batch.iter().any(|packet| {
                    matches!(packet, packets::PlayClientbound::PlayerInfoRemove(remove)
                        if remove.profile_ids.contains(&profile_id))
                });
                for packet in batch {
                    primary.apply_observed_play_packet(packet).await?;
                }
                if removed {
                    return Ok::<_, anyhow::Error>(());
                }
            }
        }
    })
    .await
    .context("refused player was not removed from the primary session")?
}

mod interaction_replay;
pub(crate) use interaction_replay::*;

#[cfg(test)]
mod tests;
