//! Shared fixtures for the server-world tests.
//!
//! The topic modules below each hold the tests for one surface; what stays
//! here is what more than one of them reaches for.

use std::collections::BTreeMap;
use std::sync::Arc;

use bbb_facts::container_inputs::ContainerInput;
use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID;
use bbb_protocol::entity_synched_data::{
    VANILLA_ENTITY_SHARED_FLAGS_DATA_ID, VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN,
    VANILLA_ENTITY_SHARED_FLAG_SPRINTING,
};
use bbb_protocol::game_rules::GameRuleValue;
use bbb_protocol::game_types::GameType;
use bbb_protocol::packets::{
    decode_play_clientbound, encode_play_clientbound_level_chunk_with_light, ArgumentSignature,
    ArgumentSignatures, AttackEntity, BlockDestruction, BlockEntityTagQuery, BlockHitResult,
    BlockPos, BlockUpdate, ChangeGameModeCommand, ChatCommandSigned, ChatMessage,
    ClientCommandAction, ClientInformation, CommandBlockMode, CommandSuggestionRequest,
    ContainerButtonClick, ContainerClick, ContainerCloseRequest, ContainerSlotStateChanged,
    CustomClickAction, CustomPayload, CustomPayloadBody, DamageEvent, DataComponentPatchSummary,
    Direction, EditBook, EntityDataValueKind, EntityEvent, EntityTagQuery, GameProfile,
    HashedStack, InteractEntity, InteractionHand, ItemStackSummary, JigsawGenerate, JigsawJoint,
    LastSeenMessagesUpdate, LevelChunkWithLight, MessageSignature, MovePlayer, MoveVehicleCommand,
    PaddleBoat, PickItemFromBlock, PickItemFromEntity, PlaceRecipeCommand, PlayClientbound,
    PlayServerbound, PlayerAbilities, PlayerAction, PlayerActionKind, PlayerCommand, PlayerInput,
    PlayerRotation, PongResponse, RecipeBookChangeSettingsCommand, RecipeBookSeenRecipeCommand,
    RecipeBookType, RecipeDisplayId, RenameItem, SeenAdvancements, SelectAdvancementsTab,
    SelectBundleItem, SelectTradeCommand, SetBeacon, SetCommandBlock, SetCommandMinecart,
    SetCreativeModeSlot, SetGameRule, SetGameRuleEntry, SetJigsawBlock, SetStructureBlock,
    SetTestBlock, SignUpdate, SoundEventHolder, SoundSource, SpectateEntity, StructureBlockMirror,
    StructureBlockMode, StructureBlockRotation, StructureBlockUpdateKind, TeleportToEntity,
    TestBlockMode, TestInstanceBlockAction, TestInstanceBlockActionKind, TestInstanceBlockRotation,
    UseItem, UseItemOn, Vec3d, Vec3i, PLAYER_RELATIVE_DELTA_X, PLAYER_RELATIVE_DELTA_Y,
    PLAYER_RELATIVE_DELTA_Z, PLAYER_RELATIVE_X, PLAYER_RELATIVE_X_ROT, PLAYER_RELATIVE_Y,
    PLAYER_RELATIVE_Y_ROT, PLAYER_RELATIVE_Z,
};
use bbb_protocol::player_command_actions::PlayerCommandAction;
use bbb_protocol::player_info_actions::PlayerInfoAction;
use bbb_protocol::resource_pack_response_actions::ResourcePackResponseAction;
use bbb_protocol::sound_event_types::{
    vanilla_sound_event_registry_id, vanilla_sound_event_resource_id,
};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use uuid::Uuid;

use crate::admin::{
    AdminEnvelope, AdminOutcome, AdminRequest, AdminResult, AdminScope, AdminSetting, NameAndId,
    OperatorEntry, OperatorRequest, PlayerRef, StoredUserLists, UserBanRequest,
};
use crate::block_behavior::SET_BLOCK_UPDATE_CLIENTS;
use crate::block_entity::BlockEntity;
use crate::block_entity::BlockEntityPayload;
use crate::chunk::ChunkStore;
use crate::commands::CommandPermission;
use crate::connection::ClientboundPlayPacket;
use crate::connection::ConnectionId;
use crate::entity::EntityData;
use crate::fluid::SET_BLOCK_UPDATE_ALL;
use crate::inventory::{MENU_CONTAINER_ID, MENU_SLOT_COUNT};
use crate::level_event::RecordedGameEvent;
use crate::level_event::VanillaGameEvent;
use crate::mailbox::world_command_mailbox;
use crate::mailbox::CommandEnvelope;
use crate::mailbox::MailboxClassLimits;
use crate::mailbox::WorldCommandReceiver;
use crate::mailbox::WorldCommandSendError;
use crate::mailbox::WorldCommandSender;
use crate::mailbox::WorldMailboxLimits;
use crate::mailbox::WorldProducerClass;
use crate::world::BlockSnapshot;
use crate::world::JoinContext;
use crate::world::PlayerSnapshot;
use crate::world::ReloadedDatapacks;
use crate::world::SaveTrigger;
use crate::world::ServerWorld;
use crate::world::ServerWorldConfig;
use crate::world::TickResult;
use crate::world::WorldCloseReason;
use crate::world::WorldCommand;
use crate::world::WorldEvent;
use crate::world::{RecipeBookTypeSettings, SPAWN_Y};
use crate::world_creation::created_save;
use crate::world_creation::default_overworld_level_source;
use bbb_server_datapack::registry_data_loader::DatapackLoadLimits;
use bbb_server_persistence::Java26ChunkContext;
use bbb_server_persistence::Java26ChunkData;
use bbb_server_persistence::Java26ChunkStatus;
use bbb_server_persistence::Java26CreatedPlayer;
use bbb_server_persistence::Java26CreatedSave;
use bbb_server_persistence::Java26EntityChunkData;
use bbb_server_persistence::Java26LevelData;
use bbb_server_persistence::Java26LevelSpawn;
use bbb_server_persistence::Java26LoadedChunk;
use bbb_server_persistence::Java26LoadedDimension;
use bbb_server_persistence::Java26LoadedEntityChunk;
use bbb_server_persistence::Java26PlayerData;
use bbb_server_persistence::Java26PlayerSavePatch;
use bbb_server_persistence::Java26SaveRequest;
use bbb_server_persistence::Java26WorldCandidate;
use bbb_server_persistence::Java26WorldCreation;
use bbb_server_persistence::Java26WorldGenSettings;
use bbb_server_persistence::Java26WorldStorage;
use bbb_server_persistence::LevelDatStore;
use bbb_server_persistence::PlayerDataSource;
use bbb_server_persistence::PlayerDataStore;
use bbb_server_persistence::RegionCompression;
use bbb_server_types::fixture::FlatFixture;
use bbb_server_types::save::Java26BuiltinDimension;
use bbb_server_types::save::Java26LevelKey;
use bbb_server_types::save::NamedCompound;
use bbb_server_types::save::SaveResult;
use bbb_server_types::save::WorldSaveLimits;
use bbb_server_types::save::WorldSavePaths;
use bbb_server_types::save::JAVA_26_1_DATA_VERSION;
use bbb_server_types::save::JAVA_26_1_STORAGE_VERSION;

pub(crate) fn native_stack(
    summary: ItemStackSummary,
) -> bbb_server_types::item_stack::RuntimeItemStack {
    bbb_server_types::item_stack::RuntimeItemStack::from_summary(summary)
}

/// The `components` patch a save writes for `stack`, keyed by written name
/// (`!`-prefixed for a removal).
pub(crate) fn saved_item_components(
    world: &ServerWorld,
    stack: &bbb_server_types::item_stack::RuntimeItemStack,
) -> Vec<NbtSummaryEntry> {
    let Some(NbtSummaryValue::Compound(saved)) = stack.write(&world.item_stack_resolver()).unwrap()
    else {
        return Vec::new();
    };
    match saved.into_iter().find(|entry| entry.name == "components") {
        Some(NbtSummaryEntry {
            value: NbtSummaryValue::Compound(components),
            ..
        }) => components,
        _ => Vec::new(),
    }
}

pub(crate) fn saved_item_component(
    world: &ServerWorld,
    stack: &bbb_server_types::item_stack::RuntimeItemStack,
    name: &str,
) -> Option<NbtSummaryValue> {
    saved_item_components(world, stack)
        .into_iter()
        .find(|entry| entry.name == name)
        .map(|entry| entry.value)
}

/// A resolver that reads a tag its publication does not declare as an empty
/// tag, so fixtures may name the custom tags a level snapshot supplies later.
pub(crate) struct LenientTags<'a>(
    pub &'a dyn bbb_registry_codec::PersistentItemStackHolderResolver,
);

impl bbb_protocol::PersistentRegistryResolver for LenientTags<'_> {
    fn registry_available(&self, registry_id: &str) -> bool {
        self.0.registry_available(registry_id)
    }

    fn resource_membership(
        &self,
        registry_id: &str,
        resource_id: &str,
    ) -> bbb_protocol::PersistentRegistryMembership {
        self.0.resource_membership(registry_id, resource_id)
    }

    fn tag_members(
        &self,
        registry_id: &str,
        tag_id: &str,
    ) -> bbb_protocol::PersistentRegistryTagMembers {
        match self.0.tag_members(registry_id, tag_id) {
            bbb_protocol::PersistentRegistryTagMembers::Absent => {
                bbb_protocol::PersistentRegistryTagMembers::Present(Vec::new())
            }
            members => members,
        }
    }
}

impl bbb_registry_codec::PersistentItemStackHolderResolver for LenientTags<'_> {
    fn holder_id(&self, registry_id: &str, resource_id: &str) -> Option<i32> {
        self.0.holder_id(registry_id, resource_id)
    }

    fn holder_resource_id(&self, registry_id: &str, holder_id: i32) -> Option<String> {
        self.0.holder_resource_id(registry_id, holder_id)
    }
}

pub(crate) fn save_native_fixture_block_entity(
    entity: &crate::block_entity::BlockEntity,
) -> Vec<NbtSummaryEntry> {
    entity
        .save_with_full_metadata_with_resolver(
            crate::world::registry_runtime::item_stack_resolver_for_test(),
        )
        .expect("the fixture record saves through its registry publication")
}

pub(crate) fn load_native_fixture_block_entity(
    pos: BlockPos,
    state: i32,
    record: &[NbtSummaryEntry],
) -> crate::block_entity::BlockEntity {
    crate::block_entity::BlockEntity::load_static_with_resolver(
        pos,
        state,
        record,
        crate::world::registry_runtime::item_stack_resolver_for_test(),
    )
    .expect("the fixture record resolves through its registry publication")
    .expect("the fixture block entity loads")
}

fn level_key(dimension: Java26BuiltinDimension) -> Java26LevelKey {
    Java26LevelKey::from(dimension)
}

mod admin;
mod advancements;
mod block_destroy;
mod block_use;
mod bossbar;
mod chat_commands;
mod chest_loot;
mod chunks;
mod client_information;
mod command_block_runtime;
mod command_minecarts;
mod commands;
mod containers;
mod damage;
mod damage_and_death;
mod death_drops;
mod debug_dimension;
mod debug_subscriptions;
mod dimensions;
mod ender_dragon_fight;
mod ender_dragon_projectiles;
mod entities;
pub(crate) mod function_tags;
mod raw_command_sources;
pub(crate) use entities::{console_command, console_command_answer};
mod elytra_wall_crash;
mod entities_reference;
mod entity_transfer;
mod explosion;
mod explosion_block_runtime;
mod external_player_vehicle_control;
mod external_player_vehicle_lifecycle;
mod feedback_sounds;
mod flying_navigation;
mod gamemaster_requests;
mod global_level_events;
mod ground_navigation;
mod idle_timeout;
mod interaction;
mod inventory;
mod item_use;
mod leash;
mod leash_follow;
mod level_info;
mod locate;
mod location_changed_effects;
mod maps;
mod minecart_interaction;
mod mining;
mod mob_targets;
mod movement;
mod natural_spawn;
mod nether_portal;
mod noise_runtime;
mod official_item_corpus;
mod pause_when_empty;
mod persistence;
mod player_action;
mod player_effects;
mod player_fall_flying;
mod player_jump_exhaustion;
mod player_push;
mod player_tracking;
mod poi;
mod portal_tickets;
mod scheduled_events;
mod scoreboard;
mod session;
mod signs;
mod simulation;
mod spawn;
mod spawner_minecarts;
mod stopwatches;
mod stored_damage_reduction;
mod tag_query;
mod thunder;
mod using_item;
mod waypoints;
mod weather;
mod world_clock;

pub(crate) fn parse_advancement(
    value: &serde_json::Value,
) -> Result<
    bbb_server_datapack::advancement::Advancement,
    bbb_server_datapack::advancement::AdvancementDecodeError,
> {
    let bytes = serde_json::to_vec(value).expect("fixture JSON serializes");
    let source = bbb_registry_codec::parse_registry_json(&bytes).map_err(|error| {
        bbb_server_datapack::advancement::AdvancementDecodeError {
            message: error.to_string(),
        }
    })?;
    bbb_server_datapack::advancement::parse_advancement(&source)
}

fn profile(name: &str, uuid: u128) -> GameProfile {
    GameProfile {
        uuid: Uuid::from_u128(uuid),
        name: name.to_string(),
        properties: Vec::new(),
    }
}

pub(crate) fn mailbox() -> (WorldCommandSender, WorldCommandReceiver) {
    world_command_mailbox(WorldMailboxLimits::default()).unwrap()
}

fn enqueue(
    sender: &WorldCommandSender,
    command: WorldCommand,
) -> std::result::Result<u64, WorldCommandSendError> {
    sender.try_send(command)
}

/// Answers the batch already in flight and keeps answering until the player's
/// chunk sender runs dry, returning every position it released in order.
///
/// The caller must have a batch outstanding, which is true straight after a
/// join and after any tick whose tracking-view change queued more columns.
fn drain_chunk_deliveries(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    connection_id: u64,
) -> Vec<(i32, i32)> {
    let mut delivered = Vec::new();
    let mut idle = 0;
    let mut acknowledge = true;
    while idle < 3 {
        let tick = if acknowledge {
            play(
                sender,
                receiver,
                world,
                connection_id,
                PlayServerbound::ChunkBatchReceived {
                    desired_chunks_per_tick: 64.0,
                },
            )
        } else {
            world.tick(receiver.drain())
        };
        let batch = delivered_chunk_positions(&tick);
        acknowledge = !batch.is_empty();
        if batch.is_empty() {
            idle += 1;
        } else {
            idle = 0;
            delivered.extend(batch);
        }
    }
    delivered
}

/// Every column one tick handed to a client, in the order it was sent.
fn delivered_chunk_positions(tick: &TickResult) -> Vec<(i32, i32)> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::LevelChunkWithLight(chunk),
                ..
            } => Some((chunk.x, chunk.z)),
            _ => None,
        })
        .collect()
}

/// A fixture world whose joins land on the world spawn.
///
/// `PlayerSpawnFinder.findSpawn` (`PlayerSpawnFinder.java:50-67`) searches a
/// `respawn_radius` square around the spawn suggestion; a radius of zero is
/// the suggestion itself, which is the block these fixtures build around.
/// `tests::spawn` covers the search.
fn spawn_pinned_world(config: ServerWorldConfig) -> ServerWorld {
    let mut world = ServerWorld::new(config).expect("the fixture world");
    world.set_test_respawn_radius(0);
    world
}

/// [`spawn_pinned_world`] over a restored save.
pub(crate) fn spawn_pinned_restore(
    config: ServerWorldConfig,
    candidate: &bbb_server_persistence::Java26WorldCandidate,
) -> ServerWorld {
    let mut world = restore_stored(config, candidate);
    world.set_test_respawn_radius(0);
    world
}

/// The candidate with every stored record read into memory, for a test that
/// inspects what its save holds.
pub(crate) fn read_stored_candidate(
    mut candidate: bbb_server_persistence::Java26WorldCandidate,
) -> bbb_server_persistence::Java26WorldCandidate {
    let reader = candidate
        .column_reader
        .clone()
        .expect("an opened save reads its chunks");
    for (key, dimension) in &mut candidate.dimensions {
        for (x, z) in std::mem::take(&mut dimension.stored_chunks) {
            let column = reader.read_column(key, x, z).unwrap();
            dimension
                .chunks
                .extend(column.chunk.map(|chunk| ((x, z), chunk)));
            dimension
                .poi_chunks
                .extend(column.poi.map(|poi| ((x, z), poi)));
        }
        for (x, z) in std::mem::take(&mut dimension.stored_entity_chunks) {
            if let Some(entities) = reader.read_entity_chunk(key, x, z).unwrap() {
                dimension.entity_chunks.insert((x, z), entities);
            }
        }
    }
    for uuid in std::mem::take(&mut candidate.stored_players) {
        if let Some(entry) = reader.read_player(uuid).unwrap() {
            candidate.players.insert(uuid, entry);
        }
    }
    for uuid in std::mem::take(&mut candidate.stored_player_stats) {
        if let Some(stats) = reader.read_player_stats(uuid).unwrap() {
            candidate.player_stats.insert(uuid, stats);
        }
    }
    for uuid in std::mem::take(&mut candidate.stored_player_advancements) {
        if let Some(advancements) = reader.read_player_advancements(uuid).unwrap() {
            candidate.player_advancements.insert(uuid, advancements);
        }
    }
    for map_id in std::mem::take(&mut candidate.stored_maps) {
        if let Some(map) = reader.read_map(map_id).unwrap() {
            candidate.maps.insert(map_id, map);
        }
    }
    candidate
}

/// A restored world holding every record its save stores, as a world that
/// read its whole save at startup would.
pub(crate) fn restore_stored(
    config: ServerWorldConfig,
    candidate: &bbb_server_persistence::Java26WorldCandidate,
) -> ServerWorld {
    let mut world = ServerWorld::restore(config, candidate).expect("the restored fixture world");
    world.read_every_stored_record_for_test();
    world
}

/// Joins a player and completes the teleport-accept + player-loaded
/// handshake, draining all events. Returns the columns the join itself
/// released, which the chunk-view tests need and everything else ignores.
pub(crate) fn join_loaded(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    connection_id: u64,
    name: &str,
    uuid: u128,
) -> Vec<(i32, i32)> {
    let (joined, handshake) = join_loaded_ticks(sender, receiver, world, connection_id, name, uuid);
    let mut delivered = delivered_chunk_positions(&joined);
    delivered.extend(delivered_chunk_positions(&handshake));
    delivered
}

/// One quiet tick after the last join: a new body's first travel leaves the
/// ground alone (`Entity.java:757`), so the floor under it is found a tick
/// later, and every tracker answers that flip with an absolute sync
/// (ServerEntity.java:120-152).
pub(crate) fn settle_joined_bodies(world: &mut ServerWorld) {
    world.tick(Vec::new());
}

/// [`join_loaded`], then a full stack of the fixtures' own block in the first
/// hotbar slot.
///
/// A session the save has never seen carries the inventory `Player`'s
/// constructor builds, whose slots are all `ItemStack.EMPTY`
/// (`Inventory.java:55`), so a test whose subject needs something in hand
/// stages it here.
pub(crate) fn join_loaded_stocked(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    connection_id: u64,
    name: &str,
    uuid: u128,
) -> Vec<(i32, i32)> {
    let delivered = join_loaded(sender, receiver, world, connection_id, name, uuid);
    stock_fixture_stack(world, connection_id);
    delivered
}

/// The stack [`join_loaded_stocked`] stages, written the way the creative
/// handler writes one: menu slot 36 is the first hotbar slot.
pub(crate) fn stock_fixture_stack(world: &mut ServerWorld, connection_id: u64) {
    world.stock_menu_slot_for_test(
        ConnectionId(connection_id),
        36,
        ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
            count: 64,
            component_patch: Default::default(),
        },
    );
}

/// [`join_loaded`] keeping the two ticks it ran, for the tests that assert on
/// what the join itself published.
pub(crate) fn join_loaded_ticks(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    connection_id: u64,
    name: &str,
    uuid: u128,
) -> (TickResult, TickResult) {
    enqueue(
        sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(connection_id),
            profile: profile(name, uuid),
        },
    )
    .unwrap();
    let joined = world.tick(receiver.drain());
    let teleport_id = joined
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::PlayerPosition(update),
                ..
            } => Some(update.id),
            _ => None,
        })
        .expect("join sends the spawn teleport");
    for packet in [
        PlayServerbound::AcceptTeleportation { id: teleport_id },
        PlayServerbound::PlayerLoaded,
    ] {
        enqueue(
            sender,
            WorldCommand::Play {
                connection_id: ConnectionId(connection_id),
                packet,
            },
        )
        .unwrap();
    }
    let handshake = world.tick(receiver.drain());
    (joined, handshake)
}

/// Sends one play packet and returns the resulting tick.
fn play(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    connection_id: u64,
    packet: PlayServerbound,
) -> TickResult {
    enqueue(
        sender,
        WorldCommand::Play {
            connection_id: ConnectionId(connection_id),
            packet,
        },
    )
    .unwrap();
    world.tick(receiver.drain())
}

/// The entity id the world gave a connection.
fn entity_id_of(world: &ServerWorld, connection_id: u64) -> i32 {
    world
        .publication()
        .players
        .iter()
        .find(|player| player.connection_id == ConnectionId(connection_id))
        .expect("the connection is a player")
        .entity_id
}

#[test]
fn server_manifests_exclude_client_and_presentation_owners() {
    let manifests = [
        ("bbb-server-core", include_str!("../Cargo.toml")),
        (
            "bbb-server-types",
            include_str!("../../bbb-server-types/Cargo.toml"),
        ),
        (
            "bbb-server-datapack",
            include_str!("../../bbb-server-datapack/Cargo.toml"),
        ),
        (
            "bbb-server-worldgen",
            include_str!("../../bbb-server-worldgen/Cargo.toml"),
        ),
        (
            "bbb-server-persistence",
            include_str!("../../bbb-server-persistence/Cargo.toml"),
        ),
    ];
    for (name, manifest) in manifests {
        for forbidden in [
            "bbb-client",
            "bbb-client-assets",
            "bbb-client-audio",
            "bbb-client-clock",
            "bbb-client-net",
            "bbb-client-render",
            "bbb-client-world",
            "winit",
            "wgpu",
        ] {
            assert!(
                !manifest.lines().any(|line| {
                    let line = line.trim_start();
                    line.starts_with(forbidden)
                        && line[forbidden.len()..]
                            .starts_with(|ch: char| ch.is_whitespace() || ch == '=')
                }),
                "{name} must not depend on {forbidden}"
            );
        }
    }
}

fn nbt(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

fn nbt_strings(values: &[&str]) -> NbtSummaryValue {
    NbtSummaryValue::List(
        values
            .iter()
            .map(|value| NbtSummaryValue::string(*value))
            .collect(),
    )
}

fn nbt_float(value: f32) -> NbtSummaryValue {
    NbtSummaryValue::Float(value.to_bits())
}

fn nbt_double(value: f64) -> NbtSummaryValue {
    NbtSummaryValue::Double(value.to_bits())
}

fn nbt_bool(value: bool) -> NbtSummaryValue {
    NbtSummaryValue::Byte(i8::from(value))
}

fn nbt_vec3(values: [f64; 3]) -> NbtSummaryValue {
    NbtSummaryValue::List(values.into_iter().map(nbt_double).collect())
}

fn nbt_uuid(uuid: Uuid) -> NbtSummaryValue {
    let value = uuid.as_u128();
    NbtSummaryValue::IntArray(vec![
        (value >> 96) as i32,
        (value >> 64) as i32,
        (value >> 32) as i32,
        value as i32,
    ])
}

fn restore_fixture_level(game_time: i64) -> Java26LevelData {
    let raw = NamedCompound::new(
        "",
        vec![nbt(
            "Data",
            NbtSummaryValue::Compound(vec![
                nbt("ServerBrands", nbt_strings(&["vanilla"])),
                nbt("WasModded", NbtSummaryValue::Byte(0)),
                nbt("DataVersion", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
                nbt("GameType", NbtSummaryValue::Int(1)),
                nbt(
                    "spawn",
                    NbtSummaryValue::Compound(vec![
                        nbt("dimension", NbtSummaryValue::string("minecraft:overworld")),
                        nbt("pos", NbtSummaryValue::IntArray(vec![0, 64, 0])),
                        nbt("yaw", nbt_float(0.0)),
                        nbt("pitch", nbt_float(0.0)),
                    ]),
                ),
                nbt("Time", NbtSummaryValue::Long(game_time)),
                nbt("LastPlayed", NbtSummaryValue::Long(1_786_000_000_000)),
                nbt("LevelName", NbtSummaryValue::string("BBB restore fixture")),
                nbt("version", NbtSummaryValue::Int(JAVA_26_1_STORAGE_VERSION)),
                nbt("allowCommands", NbtSummaryValue::Byte(1)),
                nbt("initialized", NbtSummaryValue::Byte(1)),
                nbt(
                    "difficulty_settings",
                    NbtSummaryValue::Compound(vec![
                        nbt("difficulty", NbtSummaryValue::string("normal")),
                        nbt("hardcore", NbtSummaryValue::Byte(0)),
                        nbt("locked", NbtSummaryValue::Byte(0)),
                    ]),
                ),
                nbt(
                    "DataPacks",
                    NbtSummaryValue::Compound(vec![
                        nbt("Enabled", nbt_strings(&["vanilla"])),
                        nbt("Disabled", nbt_strings(&[])),
                    ]),
                ),
                nbt("enabled_features", nbt_strings(&["minecraft:vanilla"])),
                nbt(
                    "Version",
                    NbtSummaryValue::Compound(vec![
                        nbt("Name", NbtSummaryValue::string("26.1")),
                        nbt("Id", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
                        nbt("Snapshot", NbtSummaryValue::Byte(0)),
                        nbt("Series", NbtSummaryValue::string("main")),
                    ]),
                ),
            ]),
        )],
    );
    Java26LevelData::parse(raw).unwrap()
}

/// The world-gen settings a restore candidate carries.
///
/// `restore()` reads the overworld stem out of these to build the generator it
/// serves, so this cannot be an empty map any more: it is the document a
/// bbb-created save publishes, parsed the same way the loader parses one off
/// disk.
fn restore_fixture_world_gen_settings() -> Java26WorldGenSettings {
    let document =
        crate::world_creation::created_world_gen_settings_document(&FlatFixture::new().unwrap())
            .unwrap();
    bbb_server_persistence::world_gen_settings_schema::parse_java_26_1_world_gen_settings(
        document, 0,
    )
    .unwrap()
}

fn restore_chunk_context() -> Java26ChunkContext {
    Java26ChunkContext {
        expected_x: 0,
        expected_z: 0,
        min_section_y: -4,
        section_count: 24,
    }
}

fn restore_fixture_chunk_raw(sections: Vec<NbtSummaryValue>) -> NamedCompound {
    let post_processing = (0..24).map(|_| NbtSummaryValue::List(Vec::new())).collect();
    NamedCompound::new(
        "",
        vec![
            nbt("DataVersion", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
            nbt("xPos", NbtSummaryValue::Int(0)),
            nbt("yPos", NbtSummaryValue::Int(-4)),
            nbt("zPos", NbtSummaryValue::Int(0)),
            nbt("LastUpdate", NbtSummaryValue::Long(99)),
            nbt("InhabitedTime", NbtSummaryValue::Long(7)),
            nbt("Status", NbtSummaryValue::string("minecraft:full")),
            nbt("sections", NbtSummaryValue::List(sections)),
            nbt("block_entities", NbtSummaryValue::List(Vec::new())),
            nbt("block_ticks", NbtSummaryValue::List(Vec::new())),
            nbt("fluid_ticks", NbtSummaryValue::List(Vec::new())),
            nbt("PostProcessing", NbtSummaryValue::List(post_processing)),
            nbt(
                "Heightmaps",
                NbtSummaryValue::Compound(
                    [
                        "OCEAN_FLOOR",
                        "WORLD_SURFACE",
                        "MOTION_BLOCKING",
                        "MOTION_BLOCKING_NO_LEAVES",
                    ]
                    .into_iter()
                    .map(|name| nbt(name, NbtSummaryValue::LongArray(vec![0; 37])))
                    .collect(),
                ),
            ),
            nbt(
                "structures",
                NbtSummaryValue::Compound(vec![
                    nbt("starts", NbtSummaryValue::Compound(Vec::new())),
                    nbt("References", NbtSummaryValue::Compound(Vec::new())),
                ]),
            ),
        ],
    )
}

fn restore_fixture_player(
    uuid: Uuid,
    position: [f64; 3],
    rotation: [f32; 2],
    on_ground: bool,
) -> Java26PlayerData {
    restore_fixture_player_with_warden_spawn_tracker(uuid, position, rotation, on_ground, [0, 0, 0])
}

/// [`restore_fixture_player`] with the document's own `warden_spawn_tracker`
/// counters, which `ServerPlayer.readAdditionalSaveData`
/// (`ServerPlayer.java:387`) reads back.
fn restore_fixture_player_with_warden_spawn_tracker(
    uuid: Uuid,
    position: [f64; 3],
    rotation: [f32; 2],
    on_ground: bool,
    warden_spawn_tracker: [i32; 3],
) -> Java26PlayerData {
    let item = |slot: i8, id: &str, count: i32| {
        NbtSummaryValue::Compound(vec![
            nbt("Slot", NbtSummaryValue::Byte(slot)),
            nbt("id", NbtSummaryValue::string(id)),
            nbt("count", NbtSummaryValue::Int(count)),
        ])
    };
    let raw = NamedCompound::new(
        "",
        vec![
            nbt("DataVersion", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
            nbt("UUID", nbt_uuid(uuid)),
            nbt("Pos", nbt_vec3(position)),
            nbt("Motion", nbt_vec3([0.0; 3])),
            nbt(
                "Rotation",
                NbtSummaryValue::List(vec![nbt_float(rotation[0]), nbt_float(rotation[1])]),
            ),
            nbt("fall_distance", nbt_double(0.0)),
            nbt("Fire", NbtSummaryValue::Short(-20)),
            nbt("Air", NbtSummaryValue::Short(300)),
            nbt("OnGround", nbt_bool(on_ground)),
            nbt("Invulnerable", nbt_bool(false)),
            nbt("PortalCooldown", NbtSummaryValue::Int(0)),
            nbt("Health", nbt_float(20.0)),
            nbt("HurtTime", NbtSummaryValue::Short(0)),
            nbt("HurtByTimestamp", NbtSummaryValue::Int(0)),
            nbt("DeathTime", NbtSummaryValue::Short(0)),
            nbt("AbsorptionAmount", nbt_float(0.0)),
            nbt(
                "current_impulse_context_reset_grace_time",
                NbtSummaryValue::Int(0),
            ),
            nbt("attributes", NbtSummaryValue::List(Vec::new())),
            nbt("FallFlying", nbt_bool(false)),
            nbt("Brain", NbtSummaryValue::Compound(Vec::new())),
            nbt(
                "Inventory",
                NbtSummaryValue::List(vec![item(35, "minecraft:stone", 32)]),
            ),
            nbt("SelectedItemSlot", NbtSummaryValue::Int(1)),
            nbt("SleepTimer", NbtSummaryValue::Short(0)),
            nbt("XpP", nbt_float(0.5)),
            nbt("XpLevel", NbtSummaryValue::Int(1)),
            nbt("XpTotal", NbtSummaryValue::Int(7)),
            nbt("XpSeed", NbtSummaryValue::Int(4)),
            nbt("Score", NbtSummaryValue::Int(9)),
            nbt("foodLevel", NbtSummaryValue::Int(20)),
            nbt("foodTickTimer", NbtSummaryValue::Int(0)),
            nbt("foodSaturationLevel", nbt_float(5.0)),
            nbt("foodExhaustionLevel", nbt_float(0.0)),
            nbt(
                "abilities",
                NbtSummaryValue::Compound(vec![
                    nbt("invulnerable", nbt_bool(true)),
                    nbt("flying", nbt_bool(false)),
                    nbt("mayfly", nbt_bool(true)),
                    nbt("instabuild", nbt_bool(true)),
                    nbt("mayBuild", nbt_bool(true)),
                    nbt("flySpeed", nbt_float(0.05)),
                    nbt("walkSpeed", nbt_float(0.1)),
                ]),
            ),
            nbt(
                "EnderItems",
                NbtSummaryValue::List(vec![item(26, "minecraft:dirt", 1)]),
            ),
            nbt(
                "warden_spawn_tracker",
                NbtSummaryValue::Compound(vec![
                    nbt(
                        "ticks_since_last_warning",
                        NbtSummaryValue::Int(warden_spawn_tracker[0]),
                    ),
                    nbt(
                        "warning_level",
                        NbtSummaryValue::Int(warden_spawn_tracker[1]),
                    ),
                    nbt(
                        "cooldown_ticks",
                        NbtSummaryValue::Int(warden_spawn_tracker[2]),
                    ),
                ]),
            ),
            nbt("playerGameType", NbtSummaryValue::Int(1)),
            nbt("seenCredits", nbt_bool(false)),
            nbt(
                "recipeBook",
                NbtSummaryValue::Compound(vec![
                    nbt("recipes", nbt_strings(&["stone"])),
                    nbt("toBeDisplayed", nbt_strings(&["minecraft:stone"])),
                ]),
            ),
            nbt("Dimension", NbtSummaryValue::String("overworld".into())),
            nbt("spawn_extra_particles_on_fall", nbt_bool(true)),
        ],
    );
    Java26PlayerData::parse(raw, uuid).unwrap()
}

/// A save whose scenario column is `/forceload`ed, which is the shape the
/// server-world oracle's own prepared save has: one `TicketType.FORCED` ticket
/// over chunk (0, 0) at `ChunkMap.FORCED_TICKET_LEVEL`.
///
/// Without a ticket a restored world has no simulation source at all while
/// nobody is online, so nothing in it would tick -- see
/// [`a_save_with_no_ticket_and_nobody_online_simulates_nothing`].
pub(crate) fn restore_candidate(
    game_time: i64,
    chunk: Option<Java26ChunkData>,
    players: Vec<(Uuid, Java26PlayerData)>,
) -> Java26WorldCandidate {
    let mut candidate = restore_candidate_without_tickets(game_time, chunk, players);
    candidate.chunk_tickets.insert(
        level_key(Java26BuiltinDimension::Overworld),
        vec![bbb_server_persistence::Java26SavedChunkTicket {
            chunk_x: 0,
            chunk_z: 0,
            level: 31,
            ticks_left: None,
        }],
    );
    candidate
}

fn restore_candidate_without_tickets(
    game_time: i64,
    chunk: Option<Java26ChunkData>,
    players: Vec<(Uuid, Java26PlayerData)>,
) -> Java26WorldCandidate {
    let mut chunks = BTreeMap::new();
    if let Some(chunk) = chunk {
        chunks.insert(
            (0, 0),
            Java26LoadedChunk {
                timestamp: 100,
                data: chunk,
            },
        );
    }
    let mut dimensions = BTreeMap::new();
    dimensions.insert(
        level_key(Java26BuiltinDimension::Overworld),
        Java26LoadedDimension {
            chunks,
            ..Java26LoadedDimension::new(level_key(Java26BuiltinDimension::Overworld))
        },
    );
    let world_gen_settings = restore_fixture_world_gen_settings();
    let dimension_geometries =
        bbb_server_persistence::Java26DimensionGeometries::from_builtin_settings(
            &world_gen_settings,
        );
    Java26WorldCandidate {
        player_stats: std::collections::BTreeMap::new(),
        stored_player_stats: std::collections::BTreeSet::new(),
        player_advancements: std::collections::BTreeMap::new(),
        stored_player_advancements: std::collections::BTreeSet::new(),
        level: restore_fixture_level(game_time),
        world_gen_settings,
        dimension_geometries,
        dimensions,
        column_reader: None,
        players: players
            .into_iter()
            .map(|(uuid, data)| (uuid, (PlayerDataSource::Primary, data)))
            .collect(),
        stored_players: std::collections::BTreeSet::new(),
        game_rules: BTreeMap::new(),
        world_clocks: bbb_server_persistence::Java26WorldClocks::defaults(
            &bbb_protocol::world_clock_types::VANILLA_WORLD_CLOCK_TYPES
                .iter()
                .map(|entry| entry.resource_id.to_string())
                .collect::<Vec<_>>(),
        ),
        maps: BTreeMap::new(),
        stored_maps: std::collections::BTreeSet::new(),
        map_last_id: -1,
        chunk_tickets: BTreeMap::new(),
        ender_dragon_fights: BTreeMap::new(),
        raids: Java26BuiltinDimension::ALL
            .into_iter()
            .map(|dimension| (level_key(dimension), Default::default()))
            .collect(),
        world_borders: Java26BuiltinDimension::ALL
            .into_iter()
            .map(|dimension| {
                (
                    level_key(dimension),
                    bbb_server_persistence::Java26WorldBorderSettings::default(),
                )
            })
            .collect(),
        random_sequences: bbb_server_types::random_sequences_saved::RandomSequencesSaved::default(),
        scheduled_events: Vec::new(),
        stopwatches: Default::default(),
        wandering_trader: None,
        weather: bbb_server_persistence::Java26WeatherData::default(),
        scoreboard: bbb_server_persistence::Java26ScoreboardPacked::default(),
        custom_boss_events: bbb_server_persistence::Java26CustomBossEventsPacked::default(),
        command_storage: bbb_server_persistence::Java26CommandStorageNamespaces::default(),
    }
}

/// Builds the envelope every administration test uses; only the request id and
/// the scope vary.
fn admin_envelope(operation_id: u64, request_id: &str, scope: AdminScope) -> AdminEnvelope {
    AdminEnvelope {
        operation_id,
        request_id: request_id.to_string(),
        principal: "agent".to_string(),
        scope,
        deadline_tick: u64::MAX,
        now: 1_755_000_000_000,
    }
}

fn admin_outcome(tick: &TickResult) -> &AdminOutcome {
    tick.events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Admin(outcome) => Some(outcome),
            _ => None,
        })
        .expect("an admin command answers with exactly one outcome")
}

fn closed_reasons(tick: &TickResult) -> Vec<(ConnectionId, WorldCloseReason)> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Close {
                connection_id,
                reason,
            } => Some((*connection_id, reason.clone())),
            _ => None,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// World simulation
// ---------------------------------------------------------------------------

/// A section whose cells are `base` everywhere except the named overrides.
///
/// The palette is `base` followed by each override in first-use order and the
/// storage is four bits per cell, which is the width `SectionBlocks::decode`
/// reads for any palette of sixteen entries or fewer. The cell index is
/// `(y * 16 + z) * 16 + x`, the order the saved storage is written in.
type SectionCellSpec<'a> = ([usize; 3], &'a str, &'a [(&'a str, &'a str)]);

fn restore_section_with_cells(y: i8, base: &str, cells: &[SectionCellSpec<'_>]) -> NbtSummaryValue {
    let mut palette: Vec<(String, Vec<(String, String)>)> = vec![(base.to_string(), Vec::new())];
    let mut data = vec![0_i64; 256];
    for &([x, cell_y, z], name, properties) in cells {
        let properties = properties
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect::<Vec<_>>();
        let index = palette
            .iter()
            .position(|entry| entry.0 == name && entry.1 == properties)
            .unwrap_or_else(|| {
                palette.push((name.to_string(), properties));
                palette.len() - 1
            });
        assert!(palette.len() <= 16, "the fixture packs four bits per cell");
        let cell = (cell_y * 16 + z) * 16 + x;
        data[cell / 16] |= (index as i64) << ((cell % 16) * 4);
    }
    NbtSummaryValue::Compound(vec![
        nbt("Y", NbtSummaryValue::Byte(y)),
        nbt(
            "block_states",
            NbtSummaryValue::Compound(vec![
                nbt(
                    "palette",
                    NbtSummaryValue::List(
                        palette
                            .iter()
                            .map(|(name, properties)| {
                                let mut entries =
                                    vec![nbt("Name", NbtSummaryValue::string(name.clone()))];
                                if !properties.is_empty() {
                                    entries.push(nbt(
                                        "Properties",
                                        NbtSummaryValue::Compound(
                                            properties
                                                .iter()
                                                .map(|(key, value)| {
                                                    nbt(key, NbtSummaryValue::string(value.clone()))
                                                })
                                                .collect(),
                                        ),
                                    ));
                                }
                                NbtSummaryValue::Compound(entries)
                            })
                            .collect(),
                    ),
                ),
                nbt("data", NbtSummaryValue::LongArray(data)),
            ]),
        ),
        nbt(
            "biomes",
            NbtSummaryValue::Compound(vec![nbt(
                "palette",
                NbtSummaryValue::List(vec![NbtSummaryValue::string(
                    "minecraft:plains".to_string(),
                )]),
            )]),
        ),
    ])
}

/// One `SavedTick` row, in the `i`/`x`/`y`/`z`/`t`/`p` shape `SavedTick.codec`
/// reads (`SavedTick.java:28-44`).
fn restore_saved_tick(id: &str, pos: [i32; 3], delay: i32, priority: i32) -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![
        nbt("i", NbtSummaryValue::string(id)),
        nbt("x", NbtSummaryValue::Int(pos[0])),
        nbt("y", NbtSummaryValue::Int(pos[1])),
        nbt("z", NbtSummaryValue::Int(pos[2])),
        nbt("t", NbtSummaryValue::Int(delay)),
        nbt("p", NbtSummaryValue::Int(priority)),
    ])
}

fn restore_chunk_with_ticks(
    sections: Vec<NbtSummaryValue>,
    block_ticks: Vec<NbtSummaryValue>,
    fluid_ticks: Vec<NbtSummaryValue>,
) -> Java26ChunkData {
    let mut raw = restore_fixture_chunk_raw(sections);
    for entry in raw.entries_mut() {
        if entry.name == "block_ticks" {
            entry.value = NbtSummaryValue::List(block_ticks.clone());
        }
        if entry.name == "fluid_ticks" {
            entry.value = NbtSummaryValue::List(fluid_ticks.clone());
        }
    }
    Java26ChunkData::parse(raw, restore_chunk_context()).unwrap()
}

fn block_state_id(name: &str) -> i32 {
    bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        name,
        &BTreeMap::new(),
    )
    .expect("built-in Java 26.1 block state")
}

/// The live state of one cell, read out of the published overlay against the
/// generator.
fn published_block(world: &ServerWorld, pos: BlockPos) -> Option<i32> {
    world
        .publication()
        .changed_blocks
        .iter()
        .find(|snapshot| snapshot.pos == pos)
        .map(|snapshot| snapshot.block_state_id)
}

/// `minecraft:redstone_lamp` with `LIT` set either way
/// (`RedstoneLampBlock.java:18`).
fn redstone_lamp_state(lit: bool) -> i32 {
    let mut properties = BTreeMap::new();
    properties.insert(
        "lit".to_string(),
        if lit { "true" } else { "false" }.to_string(),
    );
    bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:redstone_lamp",
        &properties,
    )
    .expect("built-in Java 26.1 block state")
}

fn live_block(world: &mut ServerWorld, pos: BlockPos) -> Option<i32> {
    world.block_state(Java26BuiltinDimension::Overworld, pos)
}

/// The creative flat-world config the `src/tests` fixtures were written around.
pub(crate) fn creative_fixture_config() -> ServerWorldConfig {
    ServerWorldConfig {
        game_type: GameType::Creative,
        ..ServerWorldConfig::default()
    }
}

/// A config whose named sessions are operators from the moment they log in.
///
/// Every command above `Commands.LEVEL_ALL` needs one, and the operator list is
/// the only thing that grants a level on this server
/// (`AdminState::permission_level`).
pub(crate) fn operator_config(operators: &[(&str, u128)]) -> ServerWorldConfig {
    let mut lists = StoredUserLists::default();
    for (name, uuid) in operators {
        let user = NameAndId {
            id: Uuid::from_u128(*uuid),
            name: (*name).into(),
        };
        lists.operators.insert(
            user.id,
            OperatorEntry {
                user,
                level: CommandPermission::Owner,
                bypasses_player_limit: false,
            },
        );
    }
    ServerWorldConfig {
        admin_lists: lists,
        ..creative_fixture_config()
    }
}

/// Runs one chat command on behalf of a connected session.
pub(crate) fn chat_command(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    connection_id: u64,
    command: &str,
) -> TickResult {
    play(
        sender,
        receiver,
        world,
        connection_id,
        PlayServerbound::ChatCommand {
            command: command.to_string(),
        },
    )
}

/// Every system-chat line a tick sent, flattened the way the client resolves
/// it, paired with the session it reached.
pub(crate) fn system_chats(tick: &TickResult) -> Vec<(ConnectionId, String)> {
    let mut lines = Vec::new();
    for event in &tick.events {
        match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::SystemChat(chat),
            } => lines.push((*connection_id, chat.content.text().to_string())),
            WorldEvent::Broadcast {
                recipients,
                packet: ClientboundPlayPacket::SystemChat(chat),
            } => {
                let text = chat.content.text().to_string();
                lines.extend(recipients.iter().map(|&id| (id, text.clone())));
            }
            _ => {}
        }
    }
    lines
}

/// The message a chat packet carries before any chat-type decoration: system
/// chat as is, the disguised message, or the player message's content.
pub(crate) fn chat_content(
    packet: &ClientboundPlayPacket,
) -> Option<bbb_protocol::RichTextComponent> {
    match packet {
        ClientboundPlayPacket::SystemChat(chat) => Some((*chat.content).clone()),
        ClientboundPlayPacket::DisguisedChat(chat) => Some(chat.message.clone()),
        ClientboundPlayPacket::PlayerChat(chat) => Some(
            chat.unsigned_content
                .clone()
                .unwrap_or_else(|| bbb_protocol::RichTextComponent::plain(chat.content.clone())),
        ),
        _ => None,
    }
}

/// What a chat packet shows, decorated by its bound chat type the way the
/// client resolves it.
pub(crate) fn shown_chat(
    world: &ServerWorld,
    packet: &ClientboundPlayPacket,
) -> Option<bbb_protocol::RichTextComponent> {
    let content = chat_content(packet)?;
    let bound = match packet {
        ClientboundPlayPacket::DisguisedChat(chat) => &chat.chat_type,
        ClientboundPlayPacket::PlayerChat(chat) => &chat.chat_type,
        _ => return Some(content),
    };
    let kind = crate::world::ChatKind::ALL
        .into_iter()
        .find(|kind| {
            world
                .synchronized_registries()
                .registry_element_index("minecraft:chat_type", kind.resource_id())
                == Some(bound.chat_type_id)
        })
        .expect("a built-in chat type");
    Some(crate::world::player_chat::decorate(
        kind,
        &content,
        &bound.name,
        bound.target_name.as_ref(),
    ))
}

/// Every chat line a tick sent, shown as [`shown_chat`] resolves it, paired
/// with the session it reached.
pub(crate) fn chat_lines(world: &ServerWorld, tick: &TickResult) -> Vec<(ConnectionId, String)> {
    let mut lines = Vec::new();
    for event in &tick.events {
        match event {
            WorldEvent::Send {
                connection_id,
                packet,
            } => {
                if let Some(shown) = shown_chat(world, packet) {
                    lines.push((*connection_id, shown.text().to_string()));
                }
            }
            WorldEvent::Broadcast { recipients, packet } => {
                if let Some(shown) = shown_chat(world, packet) {
                    let text = shown.text().to_string();
                    lines.extend(recipients.iter().map(|&id| (id, text.clone())));
                }
            }
            _ => {}
        }
    }
    lines
}

/// Creates a save in `root` with the fixture's own spawn column and opens it.
pub(crate) fn created_fixture_save(root: &std::path::Path, level_name: &str) -> Java26WorldStorage {
    let _ = std::fs::remove_dir_all(root);
    std::fs::create_dir_all(root).unwrap();
    let paths = WorldSavePaths::new(root).unwrap();
    let fixture = FlatFixture::new().unwrap();
    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let column = default_overworld_level_source()
        .unwrap()
        .generate_chunk(0, 0, fixture.biome_id())
        .into_column();
    let created = created_save(
        &Java26WorldCreation {
            level_name: level_name.to_string(),
            game_type: ServerWorld::fixture_game_type_id(),
            difficulty: Difficulty::Easy.serialized_name().to_string(),
            spawn_position: ServerWorld::fixture_spawn_block(),
            last_played: 1_786_000_000_000,
        },
        &fixture,
        &[(0, 0, &column)],
    )
    .unwrap();
    storage
        .create_world(&created, RegionCompression::Zlib, 100)
        .unwrap();
    storage
}

/// The `CommonPlayerSpawnInfo.game_type` one session was logged in with.
fn login_game_type(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    connection_id: u64,
    name: &str,
    uuid: u128,
) -> i8 {
    enqueue(
        sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(connection_id),
            profile: profile(name, uuid),
        },
    )
    .unwrap();
    world
        .tick(receiver.drain())
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                connection_id: id,
                packet: ClientboundPlayPacket::Login(login),
            } if *id == ConnectionId(connection_id) => Some(login.common_spawn_info.game_type),
            _ => None,
        })
        .expect("the session logs in")
}

/// Declares the session standing still on the ground.
///
/// A session that has only just joined has never reported an on-ground flag, so
/// `Entity.onGround()` is false and `Player.getDestroySpeed` divides the mining
/// speed by five (`Player.java:611-613`). That is vanilla's answer too, and it
/// is not what these tests are about, so they say where the client is first.
fn stand_on_ground(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    connection_id: u64,
) {
    place_player(
        sender,
        receiver,
        world,
        connection_id,
        Vec3d {
            x: 0.5,
            y: SPAWN_Y,
            z: 0.5,
        },
        None,
    );
}

/// Runs one fixture placement: a `MovePlayer` a test uses to say where a
/// session is, rather than a client move under test.
///
/// `handleMovePlayer` refuses a step past the tick budget
/// (`ServerGamePacketListenerImpl.java:1103`), so the placement runs with
/// `minecraft:player_movement_check` off and puts the rule back afterwards. A
/// test whose subject *is* the movement packet sends its own.
pub(crate) fn place_player(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    connection_id: u64,
    position: Vec3d,
    rotation: Option<PlayerRotation>,
) -> TickResult {
    let rule = "minecraft:player_movement_check";
    let previous = world.game_rule_for_test(rule);
    world.set_game_rule_for_test(rule, GameRuleValue::Boolean(false));
    let ticked = play(
        sender,
        receiver,
        world,
        connection_id,
        PlayServerbound::MovePlayer(MovePlayer {
            position: Some(position),
            rotation,
            on_ground: true,
            horizontal_collision: false,
        }),
    );
    if let Some(previous) = previous {
        world.set_game_rule_for_test(rule, previous);
    }
    ticked
}

/// Every `ClientboundBlockEventPacket` one tick sent, in order.
fn block_events_sent(tick: &TickResult) -> Vec<bbb_protocol::packets::BlockEvent> {
    let mut updates = Vec::new();
    for event in &tick.events {
        match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::BlockEvent(update),
                ..
            } => updates.push(*update),
            WorldEvent::Broadcast {
                recipients,
                packet: ClientboundPlayPacket::BlockEvent(update),
            } => updates.extend(recipients.iter().map(|_| *update)),
            _ => {}
        }
    }
    updates
}

/// The `ClientboundSoundPacket`s one tick sent, in event order.
fn sounds_sent(tick: &TickResult) -> Vec<bbb_protocol::packets::SoundEvent> {
    let mut sounds = Vec::new();
    for event in &tick.events {
        match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::Sound(update),
                ..
            } => sounds.push(update.clone()),
            WorldEvent::Broadcast {
                recipients,
                packet: ClientboundPlayPacket::Sound(update),
            } => sounds.extend(recipients.iter().map(|_| update.clone())),
            _ => {}
        }
    }
    sounds
}

/// Every `ClientboundSetHealthPacket` a tick sent, paired with the session.
fn set_healths(tick: &TickResult) -> Vec<(ConnectionId, f32)> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::SetHealth(update),
            } => Some((*connection_id, update.health)),
            _ => None,
        })
        .collect()
}

fn snapshot_player(tick: &TickResult, connection_id: u64) -> &PlayerSnapshot {
    tick.snapshot
        .players
        .iter()
        .find(|player| player.connection_id == ConnectionId(connection_id))
        .expect("the session is in the snapshot")
}

/// The datapack generation every drop test needs: without loot registries a
/// block has no table to draw from, and this server reports that rather than
/// dropping nothing.
pub(crate) fn datapack_config() -> ServerWorldConfig {
    let startup = bbb_server_datapack::startup::DatapackStartup::load(
        &bbb_server_datapack::startup::DatapackStartupOptions {
            datapacks_dir: None,
            saved_data_packs: None,
            safe_mode: false,
            limits: DatapackLoadLimits::default(),
        },
    )
    .unwrap();
    ServerWorldConfig {
        datapacks: Some(Arc::new(startup)),
        ..creative_fixture_config()
    }
}

/// [`datapack_config`] plus one pack writing each `(tag, members, replace)` as
/// `data/<namespace>/tags/block/<path>.json`.
pub(crate) fn datapack_config_with_block_tags(tags: &[(&str, &[&str], bool)]) -> ServerWorldConfig {
    let root = std::env::temp_dir().join(format!("bbb-block-tags-{}", Uuid::new_v4()));
    let pack = root.join("test");
    std::fs::create_dir_all(&pack).unwrap();
    std::fs::write(
        pack.join("pack.mcmeta"),
        r#"{"pack":{"description":"block tags","pack_format":1000,"supported_formats":[0,10000]}}"#,
    )
    .unwrap();
    for (tag, members, replace) in tags {
        let (namespace, path) = tag.split_once(':').unwrap();
        let file = pack
            .join("data")
            .join(namespace)
            .join("tags/block")
            .join(format!("{path}.json"));
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(
            file,
            serde_json::to_vec(&serde_json::json!({ "replace": replace, "values": members }))
                .unwrap(),
        )
        .unwrap();
    }
    let startup = bbb_server_datapack::startup::DatapackStartup::load(
        &bbb_server_datapack::startup::DatapackStartupOptions {
            datapacks_dir: Some(root.clone()),
            saved_data_packs: None,
            safe_mode: false,
            limits: DatapackLoadLimits::default(),
        },
    )
    .unwrap();
    std::fs::remove_dir_all(root).unwrap();
    ServerWorldConfig {
        datapacks: Some(Arc::new(startup)),
        ..creative_fixture_config()
    }
}

fn datapack_config_with_enchantment_effects(
    enchantment: &str,
    effects: serde_json::Value,
) -> ServerWorldConfig {
    datapack_config_with_enchantment_environment(enchantment, effects, &[])
}

fn datapack_config_with_enchantment_environment(
    enchantment: &str,
    effects: serde_json::Value,
    biome_attributes: &[(&str, bool)],
) -> ServerWorldConfig {
    let mut config = datapack_config();
    let mut document: serde_json::Value = serde_json::from_slice(
        &config
            .datapacks
            .as_ref()
            .unwrap()
            .registries
            .element("minecraft:enchantment", enchantment)
            .unwrap()
            .source_json,
    )
    .unwrap();
    document["effects"] = effects;
    let root = std::env::temp_dir().join(format!("bbb-enchantment-override-{}", Uuid::new_v4()));
    let pack = root.join("test");
    let (namespace, name) = enchantment.split_once(':').unwrap();
    let directory = pack.join("data").join(namespace).join("enchantment");
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(pack.join("pack.mcmeta"), r#"{"pack":{"description":"enchantment conditions","pack_format":1000,"supported_formats":[0,10000]}}"#).unwrap();
    std::fs::write(
        directory.join(format!("{name}.json")),
        serde_json::to_vec(&document).unwrap(),
    )
    .unwrap();
    for (biome, evaporates) in biome_attributes {
        let mut document: serde_json::Value = serde_json::from_slice(
            &config
                .datapacks
                .as_ref()
                .unwrap()
                .registries
                .element("minecraft:worldgen/biome", biome)
                .unwrap()
                .source_json,
        )
        .unwrap();
        document["attributes"]["minecraft:gameplay/water_evaporates"] =
            serde_json::json!(evaporates);
        let (namespace, name) = biome.split_once(':').unwrap();
        let directory = pack.join("data").join(namespace).join("worldgen/biome");
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            directory.join(format!("{name}.json")),
            serde_json::to_vec(&document).unwrap(),
        )
        .unwrap();
    }
    config.datapacks = Some(Arc::new(
        bbb_server_datapack::startup::DatapackStartup::load(
            &bbb_server_datapack::startup::DatapackStartupOptions {
                datapacks_dir: Some(root.clone()),
                saved_data_packs: None,
                safe_mode: false,
                limits: DatapackLoadLimits::default(),
            },
        )
        .unwrap(),
    ));
    std::fs::remove_dir_all(root).unwrap();
    config
}

fn set_paused_test_clocks(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    end: i64,
    operation_id: u64,
) {
    for (input, expected) in [
        ("time of minecraft:the_end pause".to_string(), 1),
        ("time of minecraft:overworld pause".to_string(), 1),
        (
            format!("time of minecraft:the_end set {end}"),
            i32::try_from(end).unwrap(),
        ),
        ("time of minecraft:overworld set 6000".to_string(), 6000),
    ] {
        let result = console_command(sender, receiver, world, operation_id, &input);
        assert_eq!(
            entities::console_command_answer(&result).0,
            Some(expected),
            "{input}"
        );
    }
    assert_eq!(world.clock_total_ticks("minecraft:the_end"), Some(end));
    assert_eq!(world.clock_total_ticks("minecraft:overworld"), Some(6000));
}

fn restore_test_structure_metadata(
    chunks: &mut crate::chunk::ChunkStore,
    data: bbb_server_worldgen::noise::structure_starts::NoiseStructureChunkData,
) {
    let [x, z] = data.chunk();
    chunks.try_load_column(x, z).unwrap();
    let mut document = bbb_server_persistence::chunk_writer::full_chunk_document(
        chunks.column(x, z).unwrap(),
        x,
        z,
        &|_| Some("minecraft:plains".into()),
        chunks.min_y(),
    )
    .unwrap();
    document
        .entries_mut()
        .iter_mut()
        .find(|entry| entry.name == "structures")
        .unwrap()
        .value = data.to_nbt();
    let saved = bbb_server_persistence::Java26ChunkData::parse(
        document,
        bbb_server_persistence::Java26ChunkContext {
            expected_x: x,
            expected_z: z,
            min_section_y: chunks.min_y().div_euclid(16),
            section_count: chunks.section_count(),
        },
    )
    .unwrap();
    chunks.restore_saved_generation_metadata(&saved).unwrap();
}

/// [`datapack_config`] with one built-in loot table's body replaced, so a drop
/// test can drive a node no built-in table of that id names.
///
/// The registry is a list sorted by id (`LootRegistries`), so replacing the
/// element in place is what a winning datapack overlay would have produced: the
/// id still resolves, and every other table is the built-in one.
pub(crate) fn datapack_config_with_loot_table(
    table_id: &str,
    body: &serde_json::Value,
) -> ServerWorldConfig {
    use bbb_server_datapack::loot::{parse_loot_table, LootElementValue};

    let mut config = datapack_config();
    let mut startup: bbb_server_datapack::startup::DatapackStartup = (**config
        .datapacks
        .as_ref()
        .expect("the fixture generation is loaded"))
    .clone();
    let registry = startup
        .loot
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:loot_table")
        .expect("the fixture generation carries the loot table registry");
    let index = registry
        .elements
        .binary_search_by(|element| element.id.as_str().cmp(table_id))
        .unwrap_or_else(|_| panic!("the built-in registry carries {table_id}"));
    registry.elements[index].value =
        LootElementValue::Table(parse_loot_table(body, None).expect("the fixture table parses"));
    config.datapacks = Some(Arc::new(startup));
    config
}

/// A dandelion standing on a block of dirt, which is the smallest thing in this
/// world that `Block.dropResources` can be reached through: pulling the dirt
/// out fails `VegetationBlock.canSurvive`, so the shape update answers air and
/// `Block.updateOrDestroy` (`Block.java:223-240`) turns that into
/// `Level.destroyBlock(pos, true, null, updateLimit)`.
fn plant_on_dirt(world: &mut ServerWorld, plant_block: &str) -> BlockPos {
    let support = BlockPos { x: 1, y: 64, z: 1 };
    let plant = BlockPos { x: 1, y: 65, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        support,
        block_state_id("minecraft:dirt"),
        SET_BLOCK_UPDATE_ALL,
    ));
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        plant,
        block_state_id(plant_block),
        SET_BLOCK_UPDATE_ALL,
    ));
    plant
}

/// `minecraft:dandelion`, whose table is one item entry rolled a constant once:
/// it drops the flower itself and makes no draw at all, because
/// `LootPool.addRandomItem` skips the weighted pick when exactly one entry is
/// valid (`LootPool.java:63-94`).
fn dandelion_on_dirt(world: &mut ServerWorld) -> BlockPos {
    plant_on_dirt(world, "minecraft:dandelion")
}

/// Pulls the support out from under [`dandelion_on_dirt`].
fn remove_the_support(world: &mut ServerWorld) {
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        BlockPos { x: 1, y: 64, z: 1 },
        block_state_id("minecraft:air"),
        SET_BLOCK_UPDATE_ALL,
    ));
}

/// The `Entities` list of the one entity-region chunk a save request writes.
fn saved_entities(request: &Java26SaveRequest) -> Vec<Vec<NbtSummaryEntry>> {
    let mut records = Vec::new();
    for write in &request.entity_writes {
        let entry = write
            .document
            .entries()
            .iter()
            .find(|entry| entry.name == "Entities")
            .expect("an entity chunk document carries Entities");
        let NbtSummaryValue::List(values) = &entry.value else {
            panic!("Entities is a list");
        };
        for value in values {
            let NbtSummaryValue::Compound(fields) = value else {
                panic!("an entity record is a compound");
            };
            records.push(fields.clone());
        }
    }
    records
}

fn field<'a>(fields: &'a [NbtSummaryEntry], name: &str) -> &'a NbtSummaryValue {
    &fields
        .iter()
        .find(|entry| entry.name == name)
        .unwrap_or_else(|| panic!("the record carries {name}: {fields:?}"))
        .value
}

fn doubles(value: &NbtSummaryValue) -> Vec<f64> {
    let NbtSummaryValue::List(values) = value else {
        panic!("expected a list of doubles, got {value:?}");
    };
    values
        .iter()
        .map(|value| match value {
            // The summary keeps a double as its raw bits, which is what makes
            // the comparison exact rather than approximate.
            NbtSummaryValue::Double(bits) => f64::from_bits(*bits),
            other => panic!("expected a double, got {other:?}"),
        })
        .collect()
}

/// The `ClientboundLevelEventPacket`s one tick sent, paired with the session
/// each went to, in event order.
fn level_events_sent(tick: &TickResult) -> Vec<(ConnectionId, bbb_protocol::packets::LevelEvent)> {
    let mut sent = Vec::new();
    for event in &tick.events {
        match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::LevelEvent(update),
            } => sent.push((*connection_id, *update)),
            WorldEvent::Broadcast {
                recipients,
                packet: ClientboundPlayPacket::LevelEvent(update),
            } => sent.extend(recipients.iter().map(|&id| (id, *update))),
            _ => {}
        }
    }
    sent
}

/// Puts one session at an exact position, which is what makes the broadcast
/// radius testable.
fn stand_at(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    connection_id: u64,
    position: Vec3d,
) {
    place_player(sender, receiver, world, connection_id, position, None);
}

/// A world that can answer a loot table, run a survival session and enforce a
/// game type: the three things a `ServerPlayerGameMode.destroyBlock` needs.
fn mining_config() -> ServerWorldConfig {
    ServerWorldConfig {
        force_game_mode: true,
        ..ServerWorldConfig {
            admin_lists: operator_config(&[("miner", 1)]).admin_lists,
            ..datapack_config()
        }
    }
}

/// The live food state one session holds, read off the save the world projects.
fn live_food(world: &mut ServerWorld) -> bbb_server_types::food::FoodData {
    world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .unwrap()
        .player_patches
        .into_iter()
        .next()
        .expect("one session is online")
        .created
        .food
}

/// The item id `resource_id` names, out of the generated registry.
fn item_id(resource_id: &str) -> i32 {
    bbb_protocol::item_types::vanilla_item_type_for_resource_id(resource_id)
        .unwrap_or_else(|| panic!("the generated registry carries {resource_id}"))
        .id
}

fn right_click(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    connection_id: u64,
    pos: BlockPos,
    sequence: i32,
) -> TickResult {
    play(
        sender,
        receiver,
        world,
        connection_id,
        PlayServerbound::UseItemOn(UseItemOn {
            hand: InteractionHand::MainHand,
            hit: BlockHitResult {
                pos,
                direction: Direction::Up,
                cursor_x: 0.5,
                cursor_y: 0.5,
                cursor_z: 0.5,
                inside: false,
                world_border_hit: false,
            },
            sequence,
        }),
    )
}

fn startup_generation() -> Arc<bbb_server_datapack::startup::DatapackStartup> {
    Arc::new(
        bbb_server_datapack::startup::DatapackStartup::load(
            &bbb_server_datapack::startup::DatapackStartupOptions {
                datapacks_dir: None,
                saved_data_packs: None,
                safe_mode: false,
                limits: DatapackLoadLimits::default(),
            },
        )
        .expect("the embedded vanilla snapshot loads"),
    )
}

mod raid_saved_data;
