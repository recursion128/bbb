use super::*;

/// One `ClientboundSoundPacket` as a client can compare it.
///
/// The wire carries an absolute position and a variant seed. Neither is
/// comparable across servers — spawn coordinates are each world's own and the
/// seed is drawn from the level's random — so the position is reported as the
/// only thing about it that has to agree: whether the sound was emitted where
/// this client is standing. The peer walks onto the observer before it swings,
/// so its attack has to be.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SoundObservation {
    /// The sound's registry resource id, resolved through the built-in
    /// catalog. An id outside it is reported rather than dropped, so a server
    /// naming a sound the other does not still differs here.
    pub sound: String,
    /// `SoundSource`'s serialized name.
    pub source: String,
    pub volume: f32,
    pub pitch: f32,
    /// The sound was emitted within the packet's own eighth-of-a-block
    /// resolution of this client's own position.
    ///
    /// The observer's own pose is exact, unlike another entity's, which
    /// reaches a client through delta encoding and interpolation.
    pub at_local_player_position: bool,
}

/// One `ClientboundSoundPacket` the single-client replay heard, positioned
/// against the note block instead of against the listener.
///
/// The note is emitted at the block rather than at whoever hears it
/// (`NoteBlock.triggerEvent`, NoteBlock, plays the sound at the
/// block's own centre), so "where this client is standing" — the question
/// [`SoundObservation`] answers — says nothing here. The position is still not
/// compared directly: two servers put their spawns in different places, so the
/// only comparable fact is whether the sound came out of the cell the replay
/// put the note block in.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SoundAtBlockObservation {
    /// The sound's registry resource id, resolved through the built-in
    /// catalog, exactly as [`SoundObservation::sound`] resolves it.
    pub sound: String,
    /// `SoundSource`'s serialized name.
    pub source: String,
    pub volume: f32,
    pub pitch: f32,
    /// The sound was emitted from the note block's own cell, to the packet's
    /// own eighth-of-a-block resolution (`ClientboundSoundPacket.java:15`).
    pub at_note_block: bool,
}

/// One entity the server introduced to this client while the replay ran.
///
/// Frozen where it was first seen rather than read back at the end, and that
/// is the whole design of the observation. `ChunkMap.addEntity`
/// pairs a new tracker with every player in range
/// straight out of `addFreshEntity` — — so the `ClientboundAddEntityPacket`
/// carries the position the entity was created at, before it has ticked once.
/// Everything after that is physics and interaction: an item drop falls, and
/// after `ItemEntity.setDefaultPickUpDelay` counts
/// ten ticks down a player standing beside it takes it off the ground
/// (`Player.aiStep` collects everything inside its own box inflated by one,
/// and `ItemEntity.playerTouch` hands the stack
/// over and discards the entity). A report that read the live entity list
/// would be reporting how many ticks each server happened to take to answer
/// the replay. This reports what each server said an entity was when it said
/// it existed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpawnedEntityObservation {
    /// The entity type's registry resource id.
    ///
    /// `minecraft:entity_type` is a built-in registry, so the wire index is
    /// each side's copy of the same static table rather than something a
    /// server published — but the id is what names the mechanism, and a
    /// server that shifted the table would still have to name the same
    /// entity here.
    pub entity_type: Option<String>,
    /// Which cell the entity spawned in, as an offset from the block the drop
    /// leg broke.
    ///
    /// Not the position: `Block.popResource` offsets the
    /// item by `Mth.nextDouble(random, -0.25, 0.25)` on each axis — and on the
    /// vertical also by half of `EntityType.ITEM`'s 0.25 height
    /// — off the cell's centre, from
    /// `Level.getRandom()`. Comparing the doubles would compare two random
    /// sequences. The offsets cannot leave the cell, though: x and z land in
    /// [0.25, 0.75) and y in [0.125, 0.625) of it, so the cell is a fact both
    /// servers owe. It is reported relative to the drop leg's own block
    /// because the absolute one is each world's own spawn plus a search.
    pub cell_offset: [i32; 3],
    /// The stack the entity carries, by the item's registry resource id.
    ///
    /// Read out of `ItemEntity.DATA_ITEM` through the generated accessor, so
    /// an entity that carries no such field reports none rather than being
    /// left out of the list.
    pub item_id: Option<String>,
    /// That stack's size, or zero when the entity carries no stack at all.
    pub item_count: i32,
    /// The network id the server gave it. Each server numbers its own
    /// entities.
    pub entity_id: i32,
    /// The uuid the server generated for it, which is random on both.
    pub uuid: String,
    /// Where the spawn packet said it was, before any tick. Carries the
    /// `popResource` randomness above, so it is reported and not compared.
    pub position: [f64; 3],
    /// What `ServerEntity.getLastSentMovement` had at pairing time. The
    /// `ItemEntity` constructor seeds the horizontal
    /// components from the level random as well.
    pub delta_movement: [f64; 3],
}

/// One `ClientboundBlockEventPacket` as the differential compares it.
///
/// `ServerLevel.runBlockEvents` broadcasts one of
/// these for every queued event whose `triggerEvent` answered true, carrying
/// the block the event was queued against and the two parameters the queue
/// held. The position is not compared — it is each world's own — so what is
/// left is the block, the parameters, and whether the event landed on the cell
/// the replay put its note block in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockEventObservation {
    /// The block the event names, resolved through the client's own block
    /// registry. `ClientboundBlockEventPacket` carries a block registry id
    /// rather than a state, and `doBlockEvent`
    /// drops the event unless the block standing there still is that block.
    pub block_name: Option<String>,
    /// `ClientboundBlockEventPacket.b0`. `NoteBlock.playNote`
    /// queues literal zeros.
    pub param_a: u8,
    /// `ClientboundBlockEventPacket.b1`, queued as a literal zero for the same
    /// reason.
    pub param_b: u8,
    /// The event was queued against the cell the replay put the note block in.
    pub at_note_block: bool,
    /// How many `ClientboundSoundPacket`s the server had already sent when this
    /// event arrived.
    ///
    /// This is what pins the order the two packets are owed in.
    /// `ServerLevel.runBlockEvents` runs
    /// `doBlockEvent` — which is where `NoteBlock.triggerEvent`
    /// plays the note — and only broadcasts the event
    /// packet once that call has answered true. A server that sent the event
    /// first, or that never played the note at all, counts differently here
    /// even when both lists hold the same members. Only sounds that came off
    /// the wire are counted, for the reason
    /// [`InteractionReplayReport::sounds_heard`] gives.
    pub positioned_sounds_before: usize,
}

/// The `Entity.DATA_SHARED_FLAGS_ID` byte one player saw of another, sampled
/// after each step of the peer's movement-state replay.
///
/// Sneak and sprint have no clientbound packet of their own: the server writes
/// a shared flag and `ServerEntity.sendDirtyEntityData` flushes the byte once a
/// tick. That flush is timing-derived — a client cannot tell one tick's worth
/// of writes from two — so the replay changes exactly one bit per step and only
/// takes the next step once the byte has been seen to change. Every value
/// below is therefore the settled byte, which both servers must reach
/// regardless of how they paced the flush.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerSharedFlagsReport {
    /// What the spawn carried. `SynchedEntityData.getNonDefaultValues`
    /// (SynchedEntityData.java:92-106) omits an untouched item entirely, so a
    /// freshly joined player publishes no flags byte at all and the field's
    /// own default is what a client holds.
    pub at_join: i8,
    /// After `START_SPRINTING` (`Entity.setSprinting`).
    pub after_start_sprinting: i8,
    /// After a `ServerboundPlayerInputPacket` with `shift` set
    /// (`Entity.setShiftKeyDown`), which must leave the
    /// sprint bit alone.
    pub after_shift_key_down: i8,
    /// After `STOP_SPRINTING` followed by `START_FALL_FLYING`. The peer has no
    /// glider, so `Player.tryToStartFallFlying` fails
    /// and `stopFallFlying` sets bit 7 and
    /// clears it again — a write pair that must leave the byte exactly where
    /// stopping the sprint left it.
    pub after_stop_sprinting_and_fall_flying: i8,
    /// After a `ServerboundInteractPacket` with the secondary-action flag
    /// clear. `handleInteract` (:1819) writes it into the same crouch state the
    /// input packet does, so the interaction must clear the bit the earlier
    /// step set.
    pub after_interact: i8,
}

/// How the server paced chunk delivery.
///
/// `PlayerChunkSender` wraps every chunk it
/// releases in a batch and waits for the client's acknowledgement before
/// opening the next one; the client times the chunks between the brackets to
/// decide the rate it asks for. A chunk sent outside a batch is one the client
/// cannot pace against, so these are the shape facts both servers must agree
/// on — not the rate itself, which is wall-clock derived and never comparable.
///
/// Every field here is cumulative and only advances on a *completed* batch, so
/// sampling it mid-stream cannot make it disagree.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkDeliveryReport {
    /// Chunks that arrived while no batch was open.
    pub chunks_outside_batches: usize,
    /// Finished batches whose declared size did not match the chunks delivered
    /// between the brackets.
    pub batches_with_wrong_size: usize,
    /// Whether the session ever saw a whole batch.
    pub completed_a_batch: bool,
}

/// What the server declared as this session's authority, and what it did with
/// the client's requests to change it.
///
/// `PlayerList.placeNewPlayer` sends the world
/// difficulty and the player's abilities right after the login; both are
/// server-owned state a client can only *ask* to change. The replay asks for
/// all three changes a non-operator client can ask for — a harder difficulty, a
/// difficulty lock, and flight — and every one of them is denied or silent on
/// the official server, so the values here plus the packet counts are a
/// two-sided assertion: the join announcement must match, and the requests must
/// change nothing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionAuthorityReport {
    /// `ClientboundChangeDifficultyPacket.difficulty`, by serialized name.
    pub difficulty: String,
    /// `ClientboundChangeDifficultyPacket.locked`.
    pub difficulty_locked: bool,
    /// How many difficulty announcements arrived. Exactly one — the join
    /// announcement — proves the gamemaster gate rejected both requests, since
    /// `MinecraftServer.setDifficulty`/`setDifficultyLocked` broadcast to every
    /// player whenever they do run.
    pub difficulty_updates: usize,
    /// `ClientboundPlayerAbilitiesPacket`, as the client stored it.
    pub abilities: LocalPlayerAbilitiesState,
    /// How many ability announcements arrived. `handlePlayerAbilities` echoes
    /// nothing, so the join announcement must remain the only one.
    pub ability_updates: usize,
}

/// The difficulty the replay asks for, which must differ from every difficulty
/// the gate configures or the denial would be invisible.
pub const INTERACTION_REQUESTED_DIFFICULTY: bbb_protocol::difficulties::Difficulty =
    bbb_protocol::difficulties::Difficulty::Hard;

/// Not `Eq`: [`sounds_heard`] carries the volume and pitch the wire does,
/// which are floats.
///
/// [`sounds_heard`]: InteractionReplayReport::sounds_heard
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InteractionReplayReport {
    pub chat_content: String,
    pub chat_observed: bool,
    pub broken_block: BlockPos,
    pub broken_block_name: Option<String>,
    pub placed_block: BlockPos,
    pub placed_block_name: Option<String>,
    pub hotbar_slot: u8,
    pub hotbar_item_id: Option<i32>,
    pub hotbar_item_count: i32,
    /// The highest sequence the server acknowledged
    /// (`ackBlockChangesUpTo`/`ClientboundBlockChangedAckPacket`). The replay
    /// ends with the use in the air, whose sequence is the highest one it
    /// sends, so this is what says that handler ran.
    pub acknowledged_sequence: Option<i32>,
    /// The `/list` reply as the client flattened it.
    ///
    /// Both servers answer with the `commands.list.players` translatable
    /// component, so the probe — which does not carry a language file — sees
    /// the key followed by the component-valued arguments. Comparing that
    /// string is the differential's evidence that command dispatch, the reply
    /// component and the server's player list all agree.
    #[serde(default)]
    pub list_command_response: Option<String>,
    /// How many times that reply arrived.
    ///
    /// The replay runs `/list` unsigned once and signed once, and
    /// `performSignedChatCommand` reaches the same dispatcher the unsigned
    /// request does — so a signed request whose signatures name nothing the
    /// parse read must produce a second, identical reply. The third request,
    /// which signs an argument no command in reach declares, must produce
    /// none.
    #[serde(default)]
    pub list_command_responses: usize,
    /// How many times a signed command was refused for naming an argument the
    /// parse never read (`INVALID_COMMAND_SIGNATURE`,
    #[serde(default)]
    pub invalid_command_signature_refusals: usize,
    /// What each server completed the replay's partial commands to.
    ///
    /// `handleCustomCommandSuggestions` (:588-601) answers with the
    /// dispatcher's own `getCompletionSuggestions`, so the reply is the
    /// commands this session may run that start with what it typed, and the
    /// range of the input they replace. The replay asks only for prefixes that
    /// no command outside `/list` can complete on either side, which is what
    /// makes the two answers comparable while the two dispatchers hold
    /// different command sets.
    #[serde(default)]
    pub command_suggestions: Vec<CommandSuggestionObservation>,
    /// The timestamp the server echoed for the replay's ping request.
    ///
    /// `handlePingRequest` answers on the connection with the client's own
    /// value, so an equal echo is the session-housekeeping surface's only
    /// directly observable result. The other packets in that surface are
    /// no-ops whose evidence is that the session survives them, which the
    /// completed replay and the zero unsupported-packet assertion carry.
    #[serde(default)]
    pub ping_response_time: Option<i64>,
    /// What the cursor holds once the container session has closed.
    ///
    /// `handleContainerClick` runs `AbstractContainerMenu.doClick` and then
    /// tells the client only what its own prediction got wrong; the replay
    /// predicts nothing, so both halves of the click come back as
    /// server-authoritative packets. The close hands the carried stack back to
    /// the inventory and broadcasts nothing itself — the next
    /// `ServerPlayer.tick` is what notices, so an empty cursor here is the
    /// per-tick menu broadcast having run, and the held count above is where
    /// the stack went.
    #[serde(default)]
    pub container_click_cursor_item_id: Option<i32>,
    #[serde(default)]
    pub container_click_cursor_item_count: i32,
    /// The clicked slot after the click, which a primary pickup empties.
    #[serde(default)]
    pub container_click_slot_item_id: Option<i32>,
    #[serde(default)]
    pub container_click_slot_item_count: i32,
    /// What the hotbar slot the replay parked on holds after the two picks.
    ///
    /// The replay selects an empty slot, aims one pick at a block whose
    /// `getCloneItemStack` is empty and one at the stone it placed. The first
    /// must leave this slot alone — a server that answered it would have put a
    /// stack here through `addAndPickItem` — and the second must move the
    /// selection off it, which is what the held slot above then reads.
    #[serde(default)]
    pub pick_parked_slot_item_id: Option<i32>,
    #[serde(default)]
    pub pick_parked_slot_item_count: i32,
    /// How many times the server answered the statistics request.
    ///
    /// `ServerStatsCounter.sendStats` (:126-134) always answers, so exactly
    /// one is what both servers owe.
    #[serde(default)]
    pub award_stats_packets: usize,
    /// How many statistics that answer carried.
    ///
    /// `sendStats` sends whatever the counter marked dirty. bbb counts
    /// nothing, so it sends none where the official server sends the tick
    /// counters it has been keeping since the join. The difference is
    /// declared rather than hidden: the packet is the part both owe, and its
    /// contents are the part this server does not keep.
    #[serde(default)]
    pub awarded_statistics: usize,
    /// How many `advMode.notAllowed` refusals arrived.
    ///
    /// The two command-block editors are the only handlers in the
    /// operator-gated surface that answer at all, and they answer exactly
    /// once each. A third would mean an editor that should have been silent
    /// spoke; fewer would mean one that should have refused let the request
    /// through.
    #[serde(default)]
    pub permission_refusals: usize,
    /// What the session was told its own permission level is
    /// (`PlayerList.sendPlayerPermissionLevel`), by
    /// the client's own name for it. It is what decides every refusal above,
    /// so comparing the refusals without it would leave the reason unchecked.
    #[serde(default)]
    pub permission_level: String,
    /// How many times the server answered an advancement tab request.
    ///
    /// `PlayerAdvancements.setSelectedTab` (:273-285) answers only when the
    /// selection moved, so of the four requests the replay sends exactly two
    /// are moves: taking the root tab and clearing it again. A third would mean
    /// a server that took an id it should have dropped.
    #[serde(default)]
    pub advancement_tab_packets: usize,
    /// Which tab the session ends up holding.
    ///
    /// The last request names a loaded advancement that is not a root, which
    /// `setSelectedTab` refuses to keep — so both servers must end with the
    /// selection clear rather than with the tab the request named.
    #[serde(default)]
    pub advancement_tab_selected: Option<String>,
    /// What the book slot holds once the editor has saved and signed.
    ///
    /// `signBook` (:999-1011) transmutes the stack, so the item identity is
    /// the first thing that says the branch ran, and the written content is
    /// what it wrote. The author is the session's own profile name on both
    /// servers, so it is compared like the rest.
    #[serde(default)]
    pub book_item_id: Option<i32>,
    #[serde(default)]
    pub book_title: Option<String>,
    #[serde(default)]
    pub book_author: Option<String>,
    #[serde(default)]
    pub book_generation: Option<i32>,
    #[serde(default)]
    pub book_resolved: Option<bool>,
    #[serde(default)]
    pub book_pages: Vec<String>,
    /// What stands in the cell the replay placed a note block into.
    ///
    /// The place is the first half of the block-event leg: `BlockItem.place`
    /// runs `NoteBlock.getStateForPlacement`, so a
    /// server that placed nothing has nothing to right-click and the event can
    /// never be queued.
    #[serde(default)]
    pub note_block_name: Option<String>,
    /// That block's state properties.
    ///
    /// `instrument` is what `setInstrument` read off
    /// the neighbours at placement time, and `note` is what the right-click
    /// moved: `useWithoutItem` cycles `NoteBlock.NOTE` and writes
    /// the state back with flag 3, so the client is told the new value. Both
    /// decide what the event sounds like, which is why they are compared
    /// rather than only the block identity.
    #[serde(default)]
    pub note_block_properties: BTreeMap<String, String>,
    /// Redstone wire block identity placed by the dust leg.
    #[serde(default)]
    pub redstone_wire_name: Option<String>,
    /// That wire's state properties: the four connection sides and `POWER`.
    ///
    /// See [`INTERACTION_REDSTONE_WIRE_STATE`] for why a lone wire is a cross
    /// and why the strength is compared even though it is zero.
    #[serde(default)]
    pub redstone_wire_properties: BTreeMap<String, String>,
    /// Every `ClientboundBlockEventPacket` the replay received, in arrival
    /// order.
    ///
    /// Complete rather than filtered, for the reason
    /// [`PeerObservationReport::sounds_heard`] is: a server that broadcasts an
    /// event the other does not differs here even when it is not the note
    /// block's.
    #[serde(default)]
    pub block_events: Vec<BlockEventObservation>,
    /// Every `ClientboundSoundPacket` the replay was *sent* before it
    /// completed, in arrival order.
    ///
    /// Sent, not heard: a client produces positioned sounds of its own as
    /// well, and a note block is one of them — `NoteBlock.triggerEvent`
    /// runs on the receiving side too, off the block
    /// event. Those are this client's work on both sides of the differential,
    /// so counting them here would report two servers as agreeing about a
    /// sound neither of them sent. What the server owes is the packet:
    /// `ServerLevel.playSeededSound` broadcasts one out of the same
    /// `doBlockEvent` call whose answer decides the block event, and
    /// `runBlockEvents` broadcasts the event only
    /// after that call returned — so a report taken once the block event has
    /// arrived already holds the sound, and an empty list is a server that
    /// never sent one rather than a report taken too early.
    #[serde(default)]
    pub sounds_heard: Vec<SoundAtBlockObservation>,
    /// Entities spawned during the replay (in network-id order, excluding the local player).
    #[serde(default)]
    pub spawned_entities: Vec<SpawnedEntityObservation>,
    /// Count of [`spawned_entities`] (retained explicitly so 0 vs 1 compares cleanly).
    #[serde(default)]
    pub spawned_entity_count: usize,
    /// Whether the drop leg's carpet item entity appeared.
    #[serde(default)]
    pub drop_observed: bool,
    /// Whether `ClientboundTakeItemEntityPacket` arrived for the drop leg's item.
    #[serde(default)]
    pub drop_picked_up: bool,
    /// Item id left in the drop leg slot after pickup.
    #[serde(default)]
    pub drop_slot_item_id: Option<i32>,
    #[serde(default)]
    pub drop_slot_item_count: i32,
}

impl InteractionReplayReport {
    /// Whether the drop leg has finished: the player took the item, and the
    /// stack it carried has arrived in the slot the leg stocked.
    ///
    /// Both halves, because they are two packets. `ItemEntity.playerTouch`
    /// sends the take announcement out of the
    /// pickup itself, while the slot the stack landed in reaches the client
    /// from the per-tick menu broadcast that runs afterwards — so a wait that
    /// ended on the announcement alone would leave the slot count a race of
    /// its own.
    pub fn drop_finished(&self) -> bool {
        self.drop_picked_up && self.drop_slot_item_count == INTERACTION_DROP_SLOT_COUNT_AFTER_PICKUP
    }
}

impl SpawnedEntityObservation {
    /// Whether this is the entity the drop leg had to produce: an item entity
    /// in the cell the carpet stood in, carrying one carpet.
    ///
    /// Everything the check reads is a fact the leg owns rather than one the
    /// world decides. [`INTERACTION_DROP_CELL_OFFSET`] explains the cell,
    /// the block's loot table explains the count, and
    /// [`INTERACTION_DROP_BLOCK_NAME`] explains the item.
    pub fn is_the_leg_drop(&self) -> bool {
        self.entity_type.as_deref() == Some(INTERACTION_DROP_ENTITY_TYPE_NAME)
            && self.cell_offset == INTERACTION_DROP_CELL_OFFSET
            && self.item_id.as_deref() == Some(INTERACTION_DROP_BLOCK_NAME)
            && self.item_count == INTERACTION_DROP_ITEM_COUNT
    }
}

/// The sequence the replay's use-in-the-air acknowledges with.
///
/// It is the highest one the replay sends, and `ackBlockChangesUpTo` keeps the
/// maximum, so the acknowledgement the client ends up holding is this one
/// exactly when the use reached the handler and the block place's otherwise.
/// The two uses the note-block leg sends and the two the drop leg sends sit
/// below it, which is why it is 9 rather than 3.
pub const INTERACTION_USE_ITEM_SEQUENCE: i32 = 9;

/// The hotbar slot the note block is stocked in, in both numberings.
///
/// It is hotbar slot 5, which nothing else in the replay touches: the held
/// slot is 1, the clicked slot 2, the parked slot 3 and the book slot 4, so
/// the note block cannot move any of the results those carry. The menu
/// addresses the same slot as 41.
pub const INTERACTION_NOTE_BLOCK_INVENTORY_SLOT: u8 = 5;
pub const INTERACTION_NOTE_BLOCK_MENU_SLOT: i16 = 41;

/// The block the replay places, by its registry resource id. The creative
/// write that stocks it looks the item up by the same id rather than by a
/// number spelled here.
pub const INTERACTION_NOTE_BLOCK_NAME: &str = "minecraft:note_block";

/// `BlockStateProperties.NOTE` (BlockStateProperties.java:118), by the name
/// the wire palette serializes it under.
pub const NOTE_BLOCK_NOTE_PROPERTY: &str = "note";

/// What `NoteBlock.NOTE` reads after the replay's one right-click.
///
/// `getStateForPlacement` starts from `registerDefaultState`
/// (NoteBlock.java:51), which pins `NOTE` to 0, and `useWithoutItem` (:126)
/// cycles it once — so a server that tuned the block says 1 here and one that
/// placed a second note block instead says 0.
pub const INTERACTION_NOTE_BLOCK_TUNED_NOTE: &str = "1";

/// Item and block state expectations for the redstone-wire leg (placed lone wire becomes a cross with power 0).
pub const INTERACTION_REDSTONE_WIRE_ITEM_NAME: &str = "minecraft:redstone";
pub const INTERACTION_REDSTONE_WIRE_NAME: &str = "minecraft:redstone_wire";

/// The slot the leg stocks its dust in. Its own, for the reason every other
/// leg's is: sharing one would make this leg's result a function of another's.
pub const INTERACTION_REDSTONE_WIRE_INVENTORY_SLOT: u8 = 7;
pub const INTERACTION_REDSTONE_WIRE_MENU_SLOT: i16 = 43;

/// The five properties the placed wire has to carry, as
/// `RedStoneWireBlock.PROPERTY_BY_DIRECTION` (RedStoneWireBlock.java:51-53) and
/// `POWER` (:50) spell them.
pub const INTERACTION_REDSTONE_WIRE_STATE: [(&str, &str); 5] = [
    ("east", "side"),
    ("north", "side"),
    ("power", "0"),
    ("south", "side"),
    ("west", "side"),
];

pub const INTERACTION_DROP_BLOCK_NAME: &str = "minecraft:white_carpet";

/// The hotbar slot the drop leg's block is stocked in, in both numberings.
///
/// It is hotbar slot 6, past every slot another observation reads: the held
/// slot is 1, the clicked slot 2, the parked slot 3, the book slot 4 and the
/// note block slot 5. The menu addresses the same slot as 42.
pub const INTERACTION_DROP_INVENTORY_SLOT: u8 = 6;
pub const INTERACTION_DROP_MENU_SLOT: i16 = 42;

/// `EntityType.ITEM`'s registry resource id (EntityType.java:596-604), which
/// is what `Block.popResource` (Block.java:417-424) creates.
pub const INTERACTION_DROP_ENTITY_TYPE_NAME: &str = "minecraft:item";

/// Where the drop has to appear, as a cell offset from the block the leg
/// broke.
///
/// The carpet stands one cell above its base, and `popResource`'s random
/// offsets cannot leave the cell the block occupied — see
/// [`SpawnedEntityObservation::cell_offset`] — so the item spawns in the
/// carpet's own cell and nowhere else.
pub const INTERACTION_DROP_CELL_OFFSET: [i32; 3] = [0, 1, 0];

/// How many items that one drop carries. The loot table rolls once for one
/// item, and `popResource` makes one entity out of the stack.
pub const INTERACTION_DROP_ITEM_COUNT: i32 = 1;

/// What the drop leg's slot holds once the player has taken the drop back.
///
/// The leg stocks one carpet and the drop is one more of the same item, and
/// `Inventory.getSlotWithRemainingSpace` (Inventory.java:223-239) can only
/// put it in a slot already holding that item — so the two end up in the one
/// slot the leg owns rather than opening a slot of their own.
pub const INTERACTION_DROP_SLOT_COUNT_AFTER_PICKUP: i32 = INTERACTION_DROP_ITEM_COUNT * 2;

/// The yaw the use is aimed with, chosen outside (-180, 180] so that
/// `Mth.wrapDegrees` has something to do.
pub const INTERACTION_USE_ITEM_UNWRAPPED_Y_ROT: f32 = 450.0;

/// What the operator-gated replay asks for.
///
/// Every value is one a permitted server would act on, so a server that let
/// the request through would leave a trace: a tag query would answer with this
/// transaction id, and a command block or structure block would keep the text.
pub const PERMISSION_QUERY_TRANSACTION_ID: i32 = 4_242;
pub const PERMISSION_REFUSED_COMMAND: &str = "say bbb permission probe";
/// The rule the refused write names. 26.1 renamed the day cycle to
/// `advance_time`, and the built-in catalog is what says so.
pub const PERMISSION_REFUSED_GAME_RULE: &str = "minecraft:advance_time";
pub const PERMISSION_REFUSED_STRUCTURE: &str = "bbb:permission_probe";

/// `advMode.notAllowed` (`ServerGamePacketListenerImpl.handleSetCommandBlock`
/// ServerGamePacketListenerImpl.java:606-607 and `handleSetCommandMinecart`
/// :665-666, the only two places that file sends the key), the one refusal in
/// this surface that reaches the client. The probe resolves no translations,
/// so the key is what the flattened system message carries.
pub const COMMAND_BLOCK_NOT_ALLOWED_KEY: &str = "advMode.notAllowed";

/// What the anvil rename asks for. No session in this replay has an `AnvilMenu`
/// open, so a server that renamed anything would be renaming it in a menu that
/// is not there.
pub const INTERACTION_ANVIL_NAME: &str = "bbb anvil probe";

/// The recipe display index the seen-recipe request names.
///
/// `RecipeManager.getRecipeFromDisplay` (:179-182) bounds-checks the index, so a
/// negative one resolves to nothing on every server whatever it published — the
/// one value both sides must decline for the same reason.
pub const INTERACTION_UNRESOLVABLE_RECIPE_DISPLAY: i32 = -1;

/// The three advancement tabs the replay opens.
///
/// `handleSeenAdvancements` (:577-586) drops an id that names no advancement,
/// and `setSelectedTab` (PlayerAdvancements.java:274-285) keeps a holder only
/// when it is a root with a display — so of these three only the middle one is
/// taken, and the last clears it again. Each move is answered, which is what
/// makes the tab count below two.
pub const INTERACTION_UNKNOWN_ADVANCEMENT_TAB: &str = "bbb:not_an_advancement";
pub const INTERACTION_ROOT_ADVANCEMENT_TAB: &str = "minecraft:story/root";
pub const INTERACTION_CHILD_ADVANCEMENT_TAB: &str = "minecraft:story/mine_stone";

/// The slot the book editor works on, in both numberings.
///
/// It is hotbar slot 4, which nothing else in the replay touches: the held
/// slot, the parked slot and the clicked slot are all elsewhere, so the book's
/// result is a function of the book requests alone. `handleEditBook` (:979)
/// accepts a hotbar slot, and the menu addresses the same slot as 40.
pub const INTERACTION_BOOK_INVENTORY_SLOT: i16 = 4;
pub const INTERACTION_BOOK_MENU_SLOT: i16 = 40;

/// What the book editor writes. The page and title are compared verbatim, and
/// the author is the session's own name, which both servers take from the
/// profile it logged in with.
pub const INTERACTION_BOOK_PAGE: &str = "bbb book probe";
pub const INTERACTION_BOOK_TITLE: &str = "bbb tales";

/// The first line the sign editor sends. No server in this differential has a
/// sign block entity anywhere, so a server that kept the text would have kept
/// it somewhere no client can read either.
pub const INTERACTION_SIGN_LINE: &str = "bbb sign probe";

/// The hotbar slot the replay parks on before it picks.
///
/// Empty on both servers by the time the picks run: neither the creative
/// writes nor the container session touches it.
pub const INTERACTION_PICK_PARKED_SLOT: u8 = 3;

/// What the replay pongs with. `handlePong` (:91-92) is empty, so the value
/// only has to be one a client could send.
pub const INTERACTION_PONG_ID: i32 = 4_321;

/// The custom click action the replay sends. `MinecraftServer.handleCustomClickAction`
/// (MinecraftServer.java:2061-2063) is a debug log: the vanilla server defines
/// no built-in actions, so a namespaced id nobody registered is the whole point.
pub const INTERACTION_CUSTOM_CLICK_ACTION: &str = "bbb:click_probe";

/// The timestamp the replay pings with. Vanilla echoes it verbatim, so any
/// value works as long as both sides use the same one; a negative value also
/// proves the field is not silently treated as unsigned.
pub const INTERACTION_PING_TIME: i64 = -1_234_567_890_123;

/// The key `ListPlayersCommand.format` reports through
/// (ListPlayersCommand.java:35). The probe resolves no translations, so the
/// key is literally part of the flattened reply and marks which chat line the
/// command answered with.
pub const LIST_COMMAND_TRANSLATION_KEY: &str = "commands.list.players";

/// `ServerGamePacketListenerImpl.INVALID_COMMAND_SIGNATURE` (:237), which is
/// the whole of what a signed command whose signatures do not match the parse
/// produces.
pub const INVALID_COMMAND_SIGNATURE_TRANSLATION_KEY: &str =
    "chat.disabled.invalid_command_signature";

/// The `InventoryMenu` slot the replay clicks, and what it holds.
///
/// It is a hotbar slot the replay stocks itself, so both servers agree on what
/// is there before the click; the *selected* slot is deliberately left alone so
/// the click cannot disturb the block interactions that come before it.
pub const INTERACTION_CLICKED_MENU_SLOT: i16 = 38;
pub const INTERACTION_CLICKED_STACK_COUNT: i32 = 16;

/// What the selected slot holds once the container session has closed.
///
/// `Inventory.getSlotWithRemainingSpace` (Inventory.java:223-239) tries the
/// held slot before it scans, so the stack the close hands back tops up the
/// one the replay stocked rather than opening a new one. The sum is what makes
/// the whole session — creative write, click, close — one readable result.
pub const INTERACTION_HELD_STACK_COUNT: i32 = 32;
pub const INTERACTION_HELD_STACK_COUNT_AFTER_CLOSE: i32 =
    INTERACTION_HELD_STACK_COUNT + INTERACTION_CLICKED_STACK_COUNT;

/// One answered command-suggestion request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandSuggestionObservation {
    pub command: bbb_protocol::JavaString,
    pub start: i32,
    pub length: i32,
    pub suggestions: Vec<bbb_protocol::JavaString>,
}

impl InteractionReplayReport {
    pub fn completed(&self) -> bool {
        self.first_wave_completed()
            // The signed book has arrived, which is the last result the replay
            // produces at all.
            && self.book_item_id == Some(bbb_protocol::item_types::VANILLA_ITEM_WRITTEN_BOOK_ID)
            && self.book_title.as_deref() == Some(INTERACTION_BOOK_TITLE)
            && self.command_suggestions.len() == 2
    }

    /// Whether every result the replay's first wave produces has arrived.
    ///
    /// This is what releases the book edits, which the replay holds back so
    /// they cannot overtake the creative write that stocks the slot they
    /// depend on. See `send_interaction_book_edits`.
    pub fn first_wave_completed(&self) -> bool {
        self.chat_observed
            && !self.chat_content.is_empty()
            && self.broken_block_name.as_deref() == Some("minecraft:air")
            && self.placed_block_name.as_deref() == Some("minecraft:stone")
            && self.hotbar_slot == 1
            && self.hotbar_item_id == Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID)
            && self.hotbar_item_count == INTERACTION_HELD_STACK_COUNT_AFTER_CLOSE
            && self.acknowledged_sequence == Some(INTERACTION_USE_ITEM_SEQUENCE)
            && self
                .list_command_response
                .as_deref()
                .is_some_and(|response| response.contains(LIST_COMMAND_TRANSLATION_KEY))
            && self.ping_response_time == Some(INTERACTION_PING_TIME)
            // The container session is last, and the stack it moved is what
            // says the whole of it landed rather than that the observation ran
            // before the answers arrived: the clicked slot is empty, the close
            // gave the cursor back, and the held slot grew by what it carried.
            && self.container_click_cursor_item_id.is_none()
            && self.container_click_slot_item_id.is_none()
            && self.pick_parked_slot_item_id.is_none()
            // Both command-block editors have answered, which orders the whole
            // operator-gated block before this report is taken.
            && self.permission_refusals == 2
            // Both signed commands have been answered, which orders the whole
            // chat surface before this report is taken.
            && self.list_command_responses == 2
            && self.invalid_command_signature_refusals == 1
            // The first completion request has been answered, which is also
            // what makes the second one — sent with the book edits — the only
            // one the client is still waiting on.
            && !self.command_suggestions.is_empty()
            && self.award_stats_packets == 1
            && !self.permission_level.is_empty()
            // Both advancement tab moves have arrived, which orders the whole
            // find-nothing block before this report is taken.
            && self.advancement_tab_packets == 2
            && self.advancement_tab_selected.is_none()
            // The block-event leg, end to end. The place has to have landed,
            // the right-click has to have moved the note, and the event the
            // right-click queues has to have been broadcast — which is also
            // what orders the sound before this report is taken, since
            // `runBlockEvents` sends the event only after `triggerEvent`
            // played it.
            && self.note_block_name.as_deref() == Some(INTERACTION_NOTE_BLOCK_NAME)
            && self
                .note_block_properties
                .get(NOTE_BLOCK_NOTE_PROPERTY)
                .map(String::as_str)
                == Some(INTERACTION_NOTE_BLOCK_TUNED_NOTE)
            && self.block_events.iter().any(|event| event.at_note_block)
            // The redstone-wire leg. The place has to have landed, and the
            // shape it landed in has to be the cross the connection walk
            // answers for an isolated wire.
            && self.redstone_wire_name.as_deref() == Some(INTERACTION_REDSTONE_WIRE_NAME)
            && INTERACTION_REDSTONE_WIRE_STATE.iter().all(|(property, value)| {
                self.redstone_wire_properties.get(*property).map(String::as_str) == Some(*value)
            })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkProbeSummary {
    pub pos: ChunkPos,
    pub state: ChunkState,
    pub heightmaps: usize,
    pub sections: usize,
    pub block_entities: usize,
    pub sky_light_arrays: usize,
    pub block_light_arrays: usize,
}

impl ChunkProbeSummary {
    pub(crate) fn from_column(column: &ChunkColumn) -> Self {
        Self {
            pos: column.pos,
            state: column.state,
            heightmaps: column.heightmaps.len(),
            sections: column.sections.len(),
            block_entities: column.block_entities.len(),
            sky_light_arrays: column.light.sky_array_count(),
            block_light_arrays: column.light.block_array_count(),
        }
    }
}

/// Separate operator-privileged session running `OPERATOR_PROBE_COMMANDS` (kept isolated so main session stays non-op).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperatorSessionReport {
    /// The name the session logged in under, which is also the name the
    /// `ops.json` fixture on each server must list. Reported rather than
    /// assumed: a fixture that named someone else would otherwise look like a
    /// dispatcher difference.
    pub username: String,
    /// What each command in [`OPERATOR_PROBE_COMMANDS`] answered with, in the
    /// order they were run.
    pub commands: Vec<CommandFeedbackObservation>,
    /// The permission-filtered command tree the session was published.
    pub command_tree: OperatorCommandTreeReport,
    /// What the two `/damage` entries in [`OPERATOR_PROBE_COMMANDS`] did to
    /// the session that ran them.
    #[serde(default)]
    pub damage: OperatorDamageReport,
}

/// Everything the hurt pipeline put on the wire for the session that asked for
/// it.
///
/// `/damage` is gated behind `Commands.LEVEL_GAMEMASTERS`
/// so this session is the only one in the replay that
/// can run it — and it names itself. That is not only convenience: every
/// packet the hurt produces then arrives on the connection that asked for it,
/// where this report is being taken, and no observation the rest of the replay
/// has already frozen can move. The main replay's report and the peer's are
/// both taken before this session even connects.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct OperatorDamageReport {
    /// The first `ClientboundSetHealthPacket` to arrive after
    /// [`OPERATOR_DAMAGE_COMMAND`] was sent, and deliberately only the first.
    ///
    /// `ServerPlayer.doTick` re-sends the packet
    /// whenever the health or the food level moved, and on the official server
    /// they keep moving after a hurt: `FoodData.tick`
    /// heals a hurt player back while `natural_health_regeneration` is on, and
    /// under this gate's `peaceful` difficulty `ServerPlayer.tickRegeneration`
    /// heals another point every twenty ticks.
    /// This server models no `FoodData` at all, so the two sides' *streams*
    /// cannot agree however the rule is set. The replay turns the rule off
    /// before it hits (see [`OPERATOR_REGENERATION_OFF_COMMAND`]) so that the
    /// one packet compared here is a function of the hurt rather than of which
    /// tick it landed on — and still compares only the first, so the assertion
    /// does not depend on how long the replay stays connected afterwards.
    pub first_health_update: Option<PlayerHealthObservation>,
    /// Every `ClientboundDamageEventPacket` that arrived after the same
    /// moment, in arrival order.
    ///
    /// Complete rather than filtered, for the reason
    /// [`InteractionReplayReport::block_events`] is: a server that broadcast a
    /// damage event the other did not differs here even when it is not this
    /// one.
    pub damage_events: Vec<DamageEventObservation>,
    /// Every `ClientboundSoundPacket` the server *sent* after the same moment,
    /// in arrival order.
    ///
    /// Sent, not heard, for the reason [`InteractionReplayReport::sounds_heard`]
    /// gives: `WorldStore` also produces a positioned sound of its own off the
    /// damage event, because vanilla's `LocalPlayer` plays its own hurt sound
    /// from `handleDamageEvent`. Counting those would report two servers as
    /// agreeing about a sound neither of them sent.
    pub sounds_heard: Vec<SoundObservation>,
}

/// One `ClientboundSetHealthPacket`, field for field.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PlayerHealthObservation {
    pub health: f32,
    pub food: i32,
    pub saturation: f32,
}

/// One `ClientboundDamageEventPacket` as the differential compares it.
///
/// `ServerLevel.broadcastDamageEvent` sends one out of `LivingEntity.hurtServer`
/// for every hit that took full damage, and
/// `broadcastAndSend` puts it on the hurt player's own connection as well as on
/// its trackers'.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DamageEventObservation {
    /// Whether the event named this session's own avatar. The raw entity id is
    /// each server's own numbering, so what is comparable is who it was.
    pub at_local_player: bool,
    /// `DamageType.STREAM_CODEC` writes the holder's index in the
    /// `minecraft:damage_type` registry (ClientboundDamageEventPacket.java:52).
    ///
    /// The index is reported rather than the resource id it stands for,
    /// because resolving it needs the registry the server published and this
    /// server publishes none: a session that never received
    /// `minecraft:damage_type` resolves the index through the client's own
    /// built-in catalog, which is in a different order and would answer with
    /// some other damage type. What both sides can say is the number, and the
    /// number is the whole of what the packet carries — `RegistryDataLoader`
    /// hands the ids out in `listResources` order
    /// (builds a `TreeMap`), so two
    /// servers loading the same documents owe each other the same index.
    pub source_type_id: i32,
    /// `sourceCauseId` and `sourceDirectId`, as
    /// `readOptionalEntityId`(ClientboundDamageEventPacket.java:46-48) decodes
    /// them: `-1` is the absent entity. The replay's `/damage` names no entity,
    /// so the sentinel is the whole of what these carry — and it is the only
    /// value of them that is comparable at all, since a real entity id is each
    /// server's own numbering.
    pub source_cause_id: i32,
    pub source_direct_id: i32,
    /// `Optional<Vec3>` as `DamageSource.sourcePositionRaw` gave it. The
    /// replay asks for it with absolute coordinates
    /// ([`OPERATOR_DAMAGE_SOURCE_POSITION`]), so the value is the literal the
    /// command carried on both servers rather than a coordinate either world
    /// chose.
    pub source_position: Option<[f64; 3]>,
}

/// Everything one command sent back to the source that ran it.
///
/// There is deliberately no success/failure field. Vanilla's two channels are
/// not two packets: `CommandSourceStack.sendSuccess`
/// hands the message to the source as the
/// command built it, while `sendFailure` sends
/// `Component.empty().append(message).withStyle(ChatFormatting.RED)` — so a
/// refusal is already distinguishable by its empty red root, and [`snbt`]
/// carries that root. A separate flag would be the same fact recorded twice,
/// and the second copy would be the probe's opinion rather than the wire's.
///
/// [`snbt`]: CommandMessageObservation::snbt
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandFeedbackObservation {
    /// The command as it was sent, without the leading slash
    /// `ServerboundChatCommandPacket` never carries.
    pub command: String,
    /// The messages the command produced, in arrival order. Empty is a real
    /// result rather than a missing one: while `send_command_feedback` is off,
    /// `ServerPlayer.commandSource().acceptsSuccess`
    /// is false and a successful command says nothing at all.
    pub messages: Vec<CommandMessageObservation>,
}

/// One message a command sent, in the two forms the differential compares.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandMessageObservation {
    /// The root component's `translate` key, or `None` when the root is not a
    /// translatable component — which is what every refusal looks like, since
    /// `sendFailure` wraps the message in an empty literal root.
    ///
    /// It is redundant with [`snbt`](Self::snbt) and kept anyway: a mismatch
    /// here names which message diverged, where the snbt only shows that one
    /// did.
    pub translate: Option<String>,
    /// The whole component, canonically encoded.
    ///
    /// The component is first run through `canonical_component_codec_value`,
    /// which resolves the codec shorthands two servers may legitimately choose
    /// between — a bare string for `{"text":...}`, a single object where a
    /// list is allowed — and then formatted with `canonical_snbt_string`,
    /// which sorts compound keys and keeps numeric type suffixes. What is left
    /// differs only when the components differ.
    ///
    /// This is deliberately not the flattened text the rest of the probe
    /// compares: `styled_runs_summary_text` drops `with` arguments that are
    /// numbers and inserts no separators, so a command whose whole answer is
    /// its arguments — `commands.gamerule.query`, `commands.list.players` —
    /// flattens to the same string no matter what it was told.
    pub snbt: String,
}

/// Command tree projection: exact equality on [`probed_command_nodes`] and set equality against declared roots on [`root_names`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorCommandTreeReport {
    /// Every root literal's name, sorted.
    pub root_names: Vec<String>,
    /// The roots [`probed_command_nodes`] covers, sorted. Fixed by
    /// [`OPERATOR_PROBE_COMMANDS`], so the two sides project the same set and
    /// a side missing one of them fails the session rather than the compare.
    ///
    /// [`probed_command_nodes`]: OperatorCommandTreeReport::probed_command_nodes
    pub probed_roots: Vec<String>,
    /// Every node reachable from those roots, sorted by path.
    pub probed_command_nodes: Vec<CommandTreeNodeObservation>,
}

/// One node of the published command tree, keyed by where it sits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandTreeNodeObservation {
    /// The node's full path from the root, as the literals and argument names
    /// on the way down joined by spaces — `gamerule send_command_feedback
    /// value`. This is the key the two sides are matched on, which is why the
    /// list is sorted by it: a tree that reorders its children is not a tree
    /// that changed.
    pub path: String,
    /// `literal` or `argument`. A root never appears: the projection starts at
    /// the roots' children and names each by its own path.
    pub kind: String,
    /// `NodeInspector.isExecutable`:
    /// `CommandNode.getCommand() != null`.
    pub executable: bool,
    /// `NodeInspector.isRestricted`: the node's
    /// requirement refuses a source holding no permission. It is what tells a
    /// client which of the commands it was sent it may only run because of who
    /// it is, so it is part of what the two servers owe each other.
    pub restricted: bool,
    /// The argument parser's registry id, `None` on literals.
    pub parser: Option<String>,
    /// `SuggestionProviders.getName` of a custom suggestion provider, `None`
    /// when the node completes from its own children.
    pub suggestions: Option<String>,
    /// The path of the node this one redirects to, `None` when it does not
    /// redirect. Resolved to a path rather than reported as an index because
    /// the indices are each server's own numbering of its own tree.
    pub redirect: Option<String>,
}

/// The name the operator session logs in under.
///
/// It has to differ from the main replay's: both servers refuse a second login
/// under a name already connected, and the main replay stays connected for the
/// whole probe. The `ops.json` fixture on each server lists exactly this name,
/// at `Commands.LEVEL_OWNERS` — level 4 — because [`OPERATOR_PROBE_COMMANDS`]
/// runs `/save-off` and `/save-on`, which are the only entries that need it.
///
/// The session never ops itself. A session that quietly failed to be an
/// operator would produce two empty observations that compare equal, which is
/// the exact false pass this differential exists to catch, so the probe checks
/// the published tree instead and fails the run when a root it is about to use
/// is missing.
pub const OPERATOR_PROBE_USERNAME: &str = "server-operator";

/// Refused zero-damage and landing `out_of_world` `/damage` commands (no knockback, absolute coords).
pub const OPERATOR_DAMAGE_INVULNERABLE_COMMAND: &str =
    "damage server-operator 0 minecraft:out_of_world";
pub const OPERATOR_DAMAGE_COMMAND: &str =
    "damage server-operator 4 minecraft:out_of_world at 1.5 65.5 2.5";

/// The rule the replay turns off before it hurts anybody.
///
/// Both of the official server's regeneration paths read it —
/// `FoodData.tick` and, under this gate's `peaceful`
/// difficulty, `ServerPlayer.tickRegeneration` (ServerPlayer.java:739) — and
/// the second heals on `tickCount % 20 == 0`, so with the rule on the
/// health the first `ClientboundSetHealthPacket` after a hurt carries depends
/// on which tick the hurt landed on. It is turned off rather than turned off
/// and back on: the packet this leg reads settles after the command list has
/// run, and putting the heal back in front of it would put the same coin flip
/// back with it.
pub const OPERATOR_REGENERATION_OFF_COMMAND: &str = "gamerule natural_health_regeneration false";

/// Ordered deterministic operator commands (omitting world-seed/mutating-disconnect/async roots; ends by testing `send_command_feedback`).
pub const OPERATOR_PROBE_COMMANDS: &[&str] = &[
    // An operator-only success with no arguments: proves the session can run a
    // LEVEL_ADMINS command at all, and that an empty ban list reports
    // `commands.banlist.none` rather than a zero-length list
    "banlist",
    // The same on a literal child rather than on the root, and on the other
    // empty-list branch.
    "whitelist list",
    // The only entry that answers twice: `SaveAllCommand.saveAll`
    // reports the save it is starting and
    // the one it finished. Nothing else in this list proves the two
    // sides put a command's messages in the same order, because nothing else
    // in it sends two.
    "save-all",
    // A refusal. `OpCommand.opPlayers` counts the
    // targets it changed and throws `commands.op.failed` when it changed none,
    // so opping the session that is already an operator is a failure that
    // changes nothing — and one only an operator can reach, since a source
    // without LEVEL_ADMINS is never sent the `op` node to parse.
    "op server-operator",
    // The only entry whose answer is a component the server assembled out of
    // several: `ListPlayersCommand.format`
    // fills `commands.list.players` with the player count, the seat count and
    // `ComponentUtils.formatList` over every
    // player's name — an empty root with the gray `", "` literal of
    // between the entries. Each name is `Player.getDisplayName`
    // the name, a `/tell` suggestion, the name again
    // as an insertion, and the entity hover of `Entity.createHoverEvent`
    // which is the one place in this list where the
    // two servers' offline uuid for the same name has to agree.
    //
    // Neither number is a property of how the run went. Both servers are given
    // the same seat count, and a seat is spent at login on the players already
    // in the list, so this session getting in at all
    // is what says the replay's is the only other one left.
    "list",
    // A query answered by a nested translatable rather than by a string:
    // `commands.difficulty.query` is built over `Difficulty.getDisplayName`
    // which is itself a translatable.
    // Both servers are configured with the same
    // difficulty and the main replay's request to change it was refused for
    // want of permission, so what comes back is what they were configured with.
    "difficulty",
    // The same difficulty again, which is a refusal — and the only refusal in
    // this list that carries an argument. `setDifficulty`
    // compares the world's difficulty against
    // the one it was given before it writes anything and throws
    // `commands.difficulty.failure` over the serialized name, so
    // asking for the difficulty the world already has changes nothing. The
    // entry above is what proves this is that difficulty.
    "difficulty peaceful",
    // An operator-only refusal that has to get past an argument check to be
    // reached at all. `PardonIpCommand.unban`
    // refuses a word that is not an address with `commands.pardonip.invalid`
    // and a well-formed one the ban list does not hold with
    // `commands.pardonip.failed`, so landing on the second is only possible if
    // both servers agreed the first check passed. Nothing is written on that
    // branch: the removal is at, past the throw.
    "pardon-ip 8.8.8.8",
    // The other side of the same boundary, on the other command that tests it.
    // `BanIpCommands.banIpOrName` treats a word that
    // is not an address as a player name, looks it up among the connected
    // players, and refuses with `commands.banip.invalid` when nobody answers to
    // it. This word is neither, so neither of the two branches that write a ban
    // and disconnect somebody is reached.
    "ban-ip not-an-address",
    // The only entry whose refusal comes from asking what the source *is*
    // rather than from counting what it named. `TeamMsgCommand` asks the source
    // for its entity and that entity for its team, and throws
    // `commands.teammsg.failed.noteam` when there is
    // none. Neither server puts a joining player on a team, so the
    // refusal lands before `resolveChatMessage` — which is what keeps
    // this entry off the chat channel the command's success would have used,
    // and why the command whose success is unusable here is still worth
    // running.
    "teammsg hi",
    // A refusal owned by an argument type rather than by a command body:
    // `MsgCommand`'s executor asks for its targets on its first line
    // and `EntityArgument.getPlayers`
    // throws `argument.entity.notfound.player`
    // when the selector matched nobody, so nothing below it is
    // reached. Naming somebody who is not connected is safe here in a way it
    // would not be in `/op`: a player selector resolves a plain name against
    // the connected players and never reaches the
    // name cache the game-profile argument resolves through.
    "msg server-nobody hi",
    // A refusal that touches nothing at all, which is what lets this list reach
    // `/tick`. `TickCommand.step` asks
    // `stepGameIfPaused`, and that returns false before it writes anything
    // while the game is not frozen. Every
    // `/tick` path that succeeds instead broadcasts a ticking-state or
    // ticking-step packet to every connected session, which is a
    // change to the replay rather than to this session — and `/tick query`
    // reports how many milliseconds this machine's ticks took, which is a
    // property of the machine.
    "tick step",
    // The same shape two literals deeper and over a different manager field:
    // `stopSprinting` answers
    // `commands.tick.sprint.stop.fail` when no sprint is running, because
    // `ServerTickRateManager.stopSprinting` returns before
    // `finishTickSprint` can report one.
    "tick sprint stop",
    // Invokes hurt pipeline after disabling regeneration; 0 damage hits refusal path before landing 4 damage.
    OPERATOR_REGENERATION_OFF_COMMAND,
    OPERATOR_DAMAGE_INVULNERABLE_COMMAND,
    OPERATOR_DAMAGE_COMMAND,
    // `/teleport` self-target exercises entity-arg executor without coordinate mismatch or cross-level drift.
    "teleport server-operator",
    "teleport server-operator server-operator",
    // `GameType.getLongDisplayName`, the display name of the
    // mode this command was given.
    //
    // It is also one of the two entries that leave the world changed — the
    // other is the regeneration rule above, which is turned off and left off:
    // `setDefaultGameType` writes the level's game type
    // and under the `force-gamemode` the
    // differential's servers both run, `getForcedGameType`
    // then answers with it, so
    // `enforceGameTypeForPlayers` moves every
    // session already connected. Both sides move identically, which is what
    // makes the answer comparable -- and it is why this whole session is asked
    // for separately from the rest of the replay: a harness that reads the save
    // afterwards would find a game type a command wrote.
    "defaultgamemode survival",
    // The query branch, before anything touches
    // the rule: both servers must report the same default for the rule the
    // rest of this list turns off and on.
    "gamerule send_command_feedback",
    // `setAutoSave` reports whether it changed anything and the command turns
    // "nothing changed" into a refusal, so the
    // same command run twice is one success and one failure — the cleanest
    // pair of channels in the dispatcher, and one that leaves no state behind
    // once its partner runs.
    "save-off",
    "save-off",
    "save-on",
    "save-on",
    // The gate goes off. This produces no message at all: the rule is already
    // false by the time `sendSuccess` asks whether the source accepts one.
    "gamerule send_command_feedback false",
    // Two successes that must now be silent, one reading state and one
    // changing it.
    "banlist",
    "save-off",
    // A refusal, which must still arrive: the failure channel is not gated.
    "save-off",
    // Silent again, and the one that puts auto-save back the way it was found.
    "save-on",
    // The gate comes back on, and this one *is* heard for the mirror image of
    // the reason the first one was not: the rule is true again before the
    // message is offered to the source.
    "gamerule send_command_feedback true",
];

/// The command the session sends after every entry of
/// [`OPERATOR_PROBE_COMMANDS`] to find out when that entry is finished.
///
/// Feedback cannot be waited for by waiting for a message: three of the
/// entries are supposed to produce none, and a wall-clock pause long enough to
/// be sure of that would be a race rather than an observation. This one is
/// answered unconditionally — `PardonCommand.pardonPlayers`
/// (PardonCommand.java:30-47) throws `commands.pardon.failed` when nothing on
/// the ban list matched, and a refusal is not gated on
/// `send_command_feedback` — and it changes nothing. Because
/// `tryHandleChat` (ServerGamePacketListenerImpl.java:1613-1622) puts every
/// chat command on the server's own task queue, the two commands run in the
/// order they were sent: the fence's refusal is the mark that everything the
/// entry before it had to say has already been said.
pub const OPERATOR_PROBE_FENCE_COMMAND: &str = "pardon server-operator";

/// The translation key [`OPERATOR_PROBE_FENCE_COMMAND`] answers with, which is
/// how its message is told apart from the observed command's.
pub const OPERATOR_PROBE_FENCE_TRANSLATION_KEY: &str = "commands.pardon.failed";
