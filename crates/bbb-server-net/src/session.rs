use std::{
    collections::{BTreeMap, HashMap},
    net::SocketAddr,
    sync::{Arc, Mutex, OnceLock, PoisonError, Weak},
};

use bbb_facts::persistent_registry_facts_26_1::vanilla_persistent_registry_membership_26_1;
use bbb_protocol::{
    codec::{offline_player_uuid, ProtocolError},
    component::{component_translatable_nbt, ComponentStyle},
    content_registry::{ContentRegistry, BBB_CONTENT_REGISTRY_CHANNEL},
    nbt_summary_to_json_value, normalize_resource_location,
    packets::{
        decode_client_intention, decode_configuration_serverbound, decode_login_serverbound,
        decode_status_serverbound, encode_configuration_brand,
        encode_configuration_code_of_conduct, encode_configuration_custom_payload,
        encode_configuration_disconnect, encode_configuration_enabled_features,
        encode_configuration_finish_clientbound, encode_configuration_keep_alive_clientbound,
        encode_configuration_registry_data, encode_configuration_resource_pack_push,
        encode_configuration_select_known_packs, encode_configuration_server_links,
        encode_configuration_update_tags, encode_login_compression, encode_login_disconnect_json,
        encode_login_finished, encode_status_pong, encode_status_response_json, ClientInformation,
        ClientIntent, ConfigurationServerbound, CustomPayloadBody, GameProfile, KnownPack,
        LoginServerbound, PublishedRegistry, RegistryDataEntry, ServerLinkEntry,
        ServerLinkKnownType, ServerLinkType, ServerLinks, StatusServerbound, UpdateEnabledFeatures,
        UpdateTags,
    },
    registry_data_types::{synchronized_registry_data_type, VanillaChatTypeIdentity},
    resource_pack_response_actions::ResourcePackResponseAction,
    world_clock_types::VANILLA_WORLD_CLOCK_TYPES,
    PersistentRegistryMembership, RichTextComponent, MC_VERSION, PROTOCOL_VERSION,
};
use bytes::Bytes;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{driver::ConnectionEvent, mailbox::MailboxLimits, Result, ServerNetError};

type CachedPacket = (i32, Bytes);

const MAX_SHARED_REGISTRY_CACHE_ENTRIES: usize = 4;

const BUILTIN_DIALOG_IDS_26_1: &[&str] = &[
    "minecraft:custom_options",
    "minecraft:quick_actions",
    "minecraft:server_links",
];

const BUILTIN_TIMELINE_IDS_26_1: &[&str] = &[
    "minecraft:day",
    "minecraft:early_game",
    "minecraft:moon",
    "minecraft:villager_schedule",
];

const BUILTIN_TEST_ENVIRONMENT_IDS_26_1: &[&str] = &["minecraft:default"];

const BUILTIN_TEST_INSTANCE_IDS_26_1: &[&str] = &["minecraft:always_pass"];

const BUILTIN_BIOME_IDS_26_1: &[&str] = &[
    "minecraft:badlands",
    "minecraft:bamboo_jungle",
    "minecraft:basalt_deltas",
    "minecraft:beach",
    "minecraft:birch_forest",
    "minecraft:cherry_grove",
    "minecraft:cold_ocean",
    "minecraft:crimson_forest",
    "minecraft:dark_forest",
    "minecraft:deep_cold_ocean",
    "minecraft:deep_dark",
    "minecraft:deep_frozen_ocean",
    "minecraft:deep_lukewarm_ocean",
    "minecraft:deep_ocean",
    "minecraft:desert",
    "minecraft:dripstone_caves",
    "minecraft:end_barrens",
    "minecraft:end_highlands",
    "minecraft:end_midlands",
    "minecraft:eroded_badlands",
    "minecraft:flower_forest",
    "minecraft:forest",
    "minecraft:frozen_ocean",
    "minecraft:frozen_peaks",
    "minecraft:frozen_river",
    "minecraft:grove",
    "minecraft:ice_spikes",
    "minecraft:jagged_peaks",
    "minecraft:jungle",
    "minecraft:lukewarm_ocean",
    "minecraft:lush_caves",
    "minecraft:mangrove_swamp",
    "minecraft:meadow",
    "minecraft:mushroom_fields",
    "minecraft:nether_wastes",
    "minecraft:ocean",
    "minecraft:old_growth_birch_forest",
    "minecraft:old_growth_pine_taiga",
    "minecraft:old_growth_spruce_taiga",
    "minecraft:pale_garden",
    "minecraft:plains",
    "minecraft:river",
    "minecraft:savanna",
    "minecraft:savanna_plateau",
    "minecraft:small_end_islands",
    "minecraft:snowy_beach",
    "minecraft:snowy_plains",
    "minecraft:snowy_slopes",
    "minecraft:snowy_taiga",
    "minecraft:soul_sand_valley",
    "minecraft:sparse_jungle",
    "minecraft:stony_peaks",
    "minecraft:stony_shore",
    "minecraft:sunflower_plains",
    "minecraft:swamp",
    "minecraft:taiga",
    "minecraft:the_end",
    "minecraft:the_void",
    "minecraft:warm_ocean",
    "minecraft:warped_forest",
    "minecraft:windswept_forest",
    "minecraft:windswept_gravelly_hills",
    "minecraft:windswept_hills",
    "minecraft:windswept_savanna",
    "minecraft:wooded_badlands",
];

static SHARED_REGISTRY_CACHE: Mutex<Vec<SharedRegistryCacheEntry>> = Mutex::new(Vec::new());

/// `KnownPack.vanilla("core")` (`KnownPack.java:14-18`) for Minecraft Java 26.1.
pub fn vanilla_core_known_pack() -> KnownPack {
    KnownPack {
        namespace: "minecraft".to_string(),
        id: "core".to_string(),
        version: MC_VERSION.to_string(),
    }
}

fn is_vanilla_core_known_pack(pack: &KnownPack) -> bool {
    pack.namespace == "minecraft" && pack.id == "core" && pack.version == MC_VERSION
}

fn cached_packet((packet_id, payload): (i32, Vec<u8>)) -> CachedPacket {
    (packet_id, Bytes::from(payload))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionPhase {
    Handshake,
    Status,
    Login,
    Configuration,
    Play,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionCloseReason {
    RemoteClosed,
    StatusHandled,
    /// The server does not reply to status requests.
    StatusIgnored,
    /// Vanilla `ReadTimeoutHandler(30)` (`ServerConnectionListener.java:67`) in
    /// Handshake or Status.
    IdleTimeout,
    LoginTimeout,
    /// `ServerCommonPacketListenerImpl.handleKeepAlive` / `keepConnectionAlive`
    /// (`ServerCommonPacketListenerImpl.java:80-88`, `:118-133`).
    KeepAliveTimeout,
    /// `PlayerList.canPlayerLogin` refused the profile during login.
    LoginRefused,
    /// Unsolicited custom query or cookie response (`DISCONNECT_UNEXPECTED_QUERY`).
    UnexpectedQuery,
    /// A required server resource pack was declined.
    ResourcePackDeclined,
    ProtocolViolation(String),
    ServerCommand,
    EventConsumerClosed,
    CommandProducerClosed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConnectionLimits {
    pub event_entries: usize,
    pub event_bytes: usize,
    pub command_entries: usize,
    pub command_bytes: usize,
}

impl Default for ConnectionLimits {
    fn default() -> Self {
        Self {
            event_entries: 64,
            event_bytes: 2 * 1024 * 1024,
            command_entries: 64,
            // Accommodates Vanilla's 8 MiB uncompressed play packet limit
            // (`CompressionDecoder.java:13-14`) even when compression is disabled.
            command_bytes: 16 * 1024 * 1024,
        }
    }
}

impl ConnectionLimits {
    pub(crate) fn validate(self) -> std::result::Result<Self, String> {
        self.event_mailbox().validate("connection event mailbox")?;
        self.command_mailbox()
            .validate("connection command mailbox")?;
        Ok(self)
    }

    pub(crate) const fn event_mailbox(self) -> MailboxLimits {
        MailboxLimits {
            entries: self.event_entries,
            bytes: self.event_bytes,
        }
    }

    pub(crate) const fn command_mailbox(self) -> MailboxLimits {
        MailboxLimits {
            entries: self.command_entries,
            bytes: self.command_bytes,
        }
    }
}

pub struct ServerConnectionConfig {
    pub status: Value,
    /// `MinecraftServer.repliesToStatus` (MinecraftServer.java:1425). When it is
    /// off, a status intention is closed without an answer
    /// (`ServerHandshakePacketListenerImpl.java:34-38`), which is a different
    /// thing from answering with an empty status.
    pub status_replies: bool,
    pub accepts_transfers: bool,
    pub brand: String,
    pub compression_threshold: Option<i32>,
    pub enabled_features: UpdateEnabledFeatures,
    pub known_packs: Vec<KnownPack>,
    /// One immutable registry transaction shared by every connection that
    /// starts from this runtime snapshot.
    pub registries: Arc<[PublishedRegistry]>,
    /// Optional unmodified built-in `minecraft:core` 26.1 registry baseline.
    /// When present, `SelectKnownPacks` (`minecraft:core` `"26.1"`) deduplication
    /// omits `raw_data` only when an entry matches this baseline byte-for-byte,
    /// keeping datapack-overridden `minecraft:*` entries intact on the wire.
    pub core_registries: Option<Arc<[PublishedRegistry]>>,
    pub tags: Arc<UpdateTags>,
    /// `bug-report-link`, sent as the one `ServerLinks` entry
    /// (`DedicatedServer.java:859-863`).
    pub bug_report_link: Option<String>,
    /// The code of conduct texts by lowercase language
    /// (`DedicatedServer.readCodeOfConducts`, `DedicatedServer.java:133-167`).
    pub code_of_conduct: Arc<BTreeMap<String, String>>,
    pub resource_pack: Option<ServerResourcePack>,
    /// The login-phase `canPlayerLogin`; absent admits every profile here and
    /// leaves the refusal to the world's own join.
    pub admission: Option<LoginAdmission>,
    /// Session-frozen modded content registry; vanilla-only omits custom configuration sync.
    pub content_registry: Arc<ContentRegistry>,
    prepared: OnceLock<Arc<PreparedConnectionConfig>>,
}

impl Clone for ServerConnectionConfig {
    fn clone(&self) -> Self {
        Self {
            status: self.status.clone(),
            status_replies: self.status_replies,
            accepts_transfers: self.accepts_transfers,
            brand: self.brand.clone(),
            compression_threshold: self.compression_threshold,
            enabled_features: self.enabled_features.clone(),
            known_packs: self.known_packs.clone(),
            registries: Arc::clone(&self.registries),
            core_registries: self.core_registries.as_ref().map(Arc::clone),
            tags: Arc::clone(&self.tags),
            bug_report_link: self.bug_report_link.clone(),
            code_of_conduct: Arc::clone(&self.code_of_conduct),
            resource_pack: self.resource_pack.clone(),
            admission: self.admission.clone(),
            content_registry: Arc::clone(&self.content_registry),
            prepared: OnceLock::new(),
        }
    }
}

impl std::fmt::Debug for ServerConnectionConfig {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ServerConnectionConfig")
            .field("status", &self.status)
            .field("status_replies", &self.status_replies)
            .field("accepts_transfers", &self.accepts_transfers)
            .field("brand", &self.brand)
            .field("compression_threshold", &self.compression_threshold)
            .field("enabled_features", &self.enabled_features)
            .field("known_packs", &self.known_packs)
            .field("registries", &self.registries)
            .field("core_registries", &self.core_registries)
            .field("tags", &self.tags)
            .field("bug_report_link", &self.bug_report_link)
            .field("code_of_conduct", &self.code_of_conduct)
            .field("resource_pack", &self.resource_pack)
            .field("admission", &self.admission)
            .finish_non_exhaustive()
    }
}

/// `MinecraftServer.ServerResourcePackInfo`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerResourcePack {
    pub id: Uuid,
    pub url: String,
    pub hash: String,
    pub required: bool,
    pub prompt: Option<RichTextComponent>,
}

pub type LoginAdmissionCheck =
    Arc<dyn Fn(&GameProfile, Option<SocketAddr>) -> Option<RichTextComponent> + Send + Sync>;

/// Answers `PlayerList.canPlayerLogin` (`PlayerList.java:334-362`) for a
/// verified profile and its remote address.
#[derive(Clone)]
pub struct LoginAdmission(pub LoginAdmissionCheck);

impl std::fmt::Debug for LoginAdmission {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("LoginAdmission")
    }
}

struct PreparedRegistrySync {
    full_frames: Box<[CachedPacket]>,
    core_dedup_frames: Box<[CachedPacket]>,
    update_tags: CachedPacket,
}

struct SharedRegistryCacheEntry {
    registries: Weak<[PublishedRegistry]>,
    tags: Weak<UpdateTags>,
    core_registries: Option<Weak<[PublishedRegistry]>>,
    sync: Arc<PreparedRegistrySync>,
}

impl SharedRegistryCacheEntry {
    fn matches(
        &self,
        registries: &Arc<[PublishedRegistry]>,
        tags: &Arc<UpdateTags>,
        core_registries: Option<&Arc<[PublishedRegistry]>>,
    ) -> bool {
        let Some(cached_registries) = self.registries.upgrade() else {
            return false;
        };
        let Some(cached_tags) = self.tags.upgrade() else {
            return false;
        };
        if !Arc::ptr_eq(&cached_registries, registries) || !Arc::ptr_eq(&cached_tags, tags) {
            return false;
        }
        match (&self.core_registries, core_registries) {
            (None, None) => true,
            (Some(cached_core), Some(core)) => cached_core
                .upgrade()
                .is_some_and(|cached_core| Arc::ptr_eq(&cached_core, core)),
            _ => false,
        }
    }

    fn is_alive(&self) -> bool {
        self.registries.strong_count() > 0
            && self.tags.strong_count() > 0
            && self
                .core_registries
                .as_ref()
                .is_none_or(|core| core.strong_count() > 0)
    }
}

struct PreparedConnectionConfig {
    status_response: CachedPacket,
    login_compression: Option<(CachedPacket, i32)>,
    brand: CachedPacket,
    server_links: Option<CachedPacket>,
    enabled_features: CachedPacket,
    select_known_packs: CachedPacket,
    offers_vanilla_core: bool,
    registry_sync: Arc<PreparedRegistrySync>,
    code_of_conduct: BTreeMap<String, CachedPacket>,
    resource_pack_push: Option<CachedPacket>,
    content_registry_packet: Option<CachedPacket>,
    configuration_finish: CachedPacket,
}

impl ServerConnectionConfig {
    pub fn new(status: Value) -> Self {
        Self {
            status,
            status_replies: true,
            accepts_transfers: false,
            brand: bbb_protocol::packets::server_connection::BBB_SERVER_BRAND.to_string(),
            compression_threshold: None,
            enabled_features: UpdateEnabledFeatures {
                features: vec!["minecraft:vanilla".to_string()],
            },
            known_packs: Vec::new(),
            registries: Arc::from(Vec::<PublishedRegistry>::new()),
            core_registries: None,
            tags: Arc::new(UpdateTags {
                registries: Vec::new(),
            }),
            bug_report_link: None,
            code_of_conduct: Arc::default(),
            resource_pack: None,
            admission: None,
            content_registry: Arc::clone(ContentRegistry::vanilla()),
            prepared: OnceLock::new(),
        }
    }

    fn server_links(&self) -> Option<ServerLinks> {
        self.bug_report_link.as_ref().map(|url| ServerLinks {
            links: vec![ServerLinkEntry {
                link_type: ServerLinkType::Known(ServerLinkKnownType::BugReport),
                url: url.clone(),
            }],
        })
    }

    fn resource_pack_push(&self, pack: &ServerResourcePack) -> Result<(i32, Vec<u8>)> {
        Ok(encode_configuration_resource_pack_push(
            pack.id,
            &pack.url,
            &pack.hash,
            pack.required,
            pack.prompt.as_ref(),
        )?)
    }

    /// Validate every startup-owned value before a listener advertises ready.
    pub fn validate(&self) -> Result<()> {
        if let Some(prepared) = self.prepared.get() {
            let _ = prepared;
            return Ok(());
        }
        self.build_prepared().map(|_| ())
    }

    fn prepared(&self) -> Result<&Arc<PreparedConnectionConfig>> {
        if let Some(prepared) = self.prepared.get() {
            return Ok(prepared);
        }
        let built = Arc::new(self.build_prepared()?);
        let _ = self.prepared.set(built);
        Ok(self.prepared.get().expect("prepared config initialized"))
    }

    fn build_prepared(&self) -> Result<PreparedConnectionConfig> {
        let status_response = cached_packet(encode_status_response_json(&self.status)?);
        let brand = cached_packet(encode_configuration_brand(&self.brand)?);
        let enabled_features = cached_packet(encode_configuration_enabled_features(
            &self.enabled_features,
        )?);
        let select_known_packs =
            cached_packet(encode_configuration_select_known_packs(&self.known_packs)?);
        let registry_sync =
            prepare_registry_sync(&self.registries, &self.tags, self.core_registries.as_ref())?;
        let server_links = self
            .server_links()
            .map(|links| encode_configuration_server_links(&links).map(cached_packet))
            .transpose()?;
        let mut code_of_conduct = BTreeMap::new();
        for (language, text) in self.code_of_conduct.iter() {
            code_of_conduct.insert(
                language.clone(),
                cached_packet(encode_configuration_code_of_conduct(text)?),
            );
        }
        let resource_pack_push = self
            .resource_pack
            .as_ref()
            .map(|pack| self.resource_pack_push(pack).map(cached_packet))
            .transpose()?;
        let login_compression = match self.compression_threshold {
            Some(threshold) if threshold < 0 => {
                return Err(ServerNetError::InvalidConfiguration(
                    "compression threshold must be non-negative or absent".to_string(),
                ));
            }
            Some(threshold) => Some((
                cached_packet(encode_login_compression(threshold)?),
                threshold,
            )),
            None => None,
        };
        let offers_vanilla_core = self.known_packs.iter().any(is_vanilla_core_known_pack);
        let content_registry_packet = if self.content_registry.is_vanilla_only() {
            None
        } else {
            let wire = self.content_registry.encode_wire();
            Some(cached_packet(encode_configuration_custom_payload(
                BBB_CONTENT_REGISTRY_CHANNEL,
                &wire,
            )?))
        };
        Ok(PreparedConnectionConfig {
            status_response,
            login_compression,
            brand,
            server_links,
            enabled_features,
            select_known_packs,
            offers_vanilla_core,
            registry_sync,
            code_of_conduct,
            resource_pack_push,
            content_registry_packet,
            configuration_finish: cached_packet(encode_configuration_finish_clientbound()),
        })
    }
}

fn prepare_registry_sync(
    registries: &Arc<[PublishedRegistry]>,
    tags: &Arc<UpdateTags>,
    core_registries: Option<&Arc<[PublishedRegistry]>>,
) -> Result<Arc<PreparedRegistrySync>> {
    {
        let mut guard = SHARED_REGISTRY_CACHE
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        guard.retain(SharedRegistryCacheEntry::is_alive);
        if let Some(index) = guard
            .iter()
            .position(|entry| entry.matches(registries, tags, core_registries))
        {
            let hit = guard.remove(index);
            let sync = Arc::clone(&hit.sync);
            guard.push(hit);
            return Ok(sync);
        }
    }

    let core_lookup = core_registries
        .map(|core| build_core_registry_lookup(core))
        .transpose()?;
    let mut full_frames = Vec::with_capacity(registries.len());
    let mut core_dedup_frames = Vec::with_capacity(registries.len());
    for registry in registries.iter() {
        let full_frame = cached_packet(encode_configuration_registry_data(
            &registry.registry,
            &registry.entries,
        )?);
        let dedup_frame = encode_core_dedup_registry_frame(registry, core_lookup.as_ref())?
            .unwrap_or_else(|| (full_frame.0, full_frame.1.clone()));
        full_frames.push(full_frame);
        core_dedup_frames.push(dedup_frame);
    }
    let update_tags = cached_packet(encode_configuration_update_tags(tags.as_ref())?);
    let sync = Arc::new(PreparedRegistrySync {
        full_frames: full_frames.into_boxed_slice(),
        core_dedup_frames: core_dedup_frames.into_boxed_slice(),
        update_tags,
    });

    {
        let mut guard = SHARED_REGISTRY_CACHE
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        guard.retain(SharedRegistryCacheEntry::is_alive);
        if !guard
            .iter()
            .any(|entry| entry.matches(registries, tags, core_registries))
        {
            if guard.len() >= MAX_SHARED_REGISTRY_CACHE_ENTRIES {
                guard.remove(0);
            }
            guard.push(SharedRegistryCacheEntry {
                registries: Arc::downgrade(registries),
                tags: Arc::downgrade(tags),
                core_registries: core_registries.map(Arc::downgrade),
                sync: Arc::clone(&sync),
            });
        }
    }

    Ok(sync)
}

type CoreRegistryLookup<'a> = HashMap<String, HashMap<String, &'a [u8]>>;

fn build_core_registry_lookup(
    core_registries: &[PublishedRegistry],
) -> Result<CoreRegistryLookup<'_>> {
    let mut lookup = HashMap::with_capacity(core_registries.len());
    for registry in core_registries {
        let registry_id = normalize_resource_location(registry.registry.clone())?;
        let entries = lookup
            .entry(registry_id)
            .or_insert_with(|| HashMap::with_capacity(registry.entries.len()));
        for entry in &registry.entries {
            if let Some(raw_data) = entry.raw_data.as_deref() {
                if raw_data.is_empty() {
                    return Err(ProtocolError::InvalidData(format!(
                        "core registry {} entry {} carries an empty nbt payload",
                        registry.registry, entry.id
                    ))
                    .into());
                }
                let entry_id = normalize_resource_location(entry.id.clone())?;
                entries.insert(entry_id, raw_data);
            }
        }
    }
    Ok(lookup)
}

/// `RegistrySynchronization.packRegistry` (`RegistrySynchronization.java:51-63`):
/// when `minecraft:core` (`26.1`) was negotiated, entries that come unmodified
/// from the built-in pack keep their arrival slot (`PackedRegistryEntry.id`)
/// while omitting their network NBT payload (`Optional.empty()`).
fn encode_core_dedup_registry_frame(
    registry: &PublishedRegistry,
    core_lookup: Option<&CoreRegistryLookup<'_>>,
) -> Result<Option<CachedPacket>> {
    let normalized_registry = normalize_resource_location(registry.registry.clone())?;
    if synchronized_registry_data_type(&normalized_registry).is_none() {
        return Ok(None);
    }

    let mut any_omitted = false;
    let mut dedup_entries = Vec::with_capacity(registry.entries.len());
    for entry in &registry.entries {
        let normalized_id = normalize_resource_location(entry.id.clone())?;
        let omit_payload = match (&entry.raw_data, core_lookup) {
            (Some(raw_data), Some(core_lookup)) => core_lookup
                .get(&normalized_registry)
                .and_then(|entries| entries.get(&normalized_id))
                .is_some_and(|core_raw| *core_raw == raw_data.as_slice()),
            (Some(_), None) => {
                is_builtin_vanilla_26_1_registry_member(&normalized_registry, &normalized_id)
            }
            (None, _) => false,
        };
        if omit_payload {
            any_omitted = true;
            dedup_entries.push(RegistryDataEntry {
                id: normalized_id,
                raw_data: None,
            });
        } else {
            dedup_entries.push(RegistryDataEntry {
                id: normalized_id,
                raw_data: entry.raw_data.clone(),
            });
        }
    }

    if !any_omitted {
        return Ok(None);
    }
    Ok(Some(cached_packet(encode_configuration_registry_data(
        &normalized_registry,
        &dedup_entries,
    )?)))
}

fn is_builtin_vanilla_26_1_registry_member(registry_id: &str, entry_id: &str) -> bool {
    if !entry_id.starts_with("minecraft:") {
        return false;
    }
    match vanilla_persistent_registry_membership_26_1(registry_id, entry_id) {
        PersistentRegistryMembership::Present => true,
        PersistentRegistryMembership::Absent => false,
        PersistentRegistryMembership::Unavailable => match registry_id {
            "minecraft:chat_type" => VanillaChatTypeIdentity::for_resource_id(entry_id).is_some(),
            "minecraft:world_clock" => VANILLA_WORLD_CLOCK_TYPES
                .iter()
                .any(|clock| clock.resource_id == entry_id),
            "minecraft:dialog" => BUILTIN_DIALOG_IDS_26_1.binary_search(&entry_id).is_ok(),
            "minecraft:timeline" => BUILTIN_TIMELINE_IDS_26_1.binary_search(&entry_id).is_ok(),
            "minecraft:test_environment" => BUILTIN_TEST_ENVIRONMENT_IDS_26_1.contains(&entry_id),
            "minecraft:test_instance" => BUILTIN_TEST_INSTANCE_IDS_26_1.contains(&entry_id),
            "minecraft:worldgen/biome" => BUILTIN_BIOME_IDS_26_1.binary_search(&entry_id).is_ok(),
            _ => false,
        },
    }
}

#[derive(Debug)]
pub(crate) struct OutboundPacket {
    pub packet_id: i32,
    pub payload: Bytes,
    pub compression_after: Option<i32>,
}

#[derive(Debug, Default)]
pub(crate) struct SessionStep {
    pub outbound: Vec<OutboundPacket>,
    pub events: Vec<ConnectionEvent>,
    pub close: Option<ConnectionCloseReason>,
}

impl SessionStep {
    fn packet(&mut self, packet: (i32, Vec<u8>)) {
        self.outbound.push(OutboundPacket {
            packet_id: packet.0,
            payload: Bytes::from(packet.1),
            compression_after: None,
        });
    }

    fn cached_packet(&mut self, packet: &CachedPacket) {
        self.outbound.push(OutboundPacket {
            packet_id: packet.0,
            payload: packet.1.clone(),
            compression_after: None,
        });
    }

    fn transition(&mut self, phase: ConnectionPhase) {
        self.events.push(ConnectionEvent::PhaseChanged { phase });
    }
}

/// `ServerConfigurationPacketListenerImpl.configurationTasks` in order
/// (`ServerConfigurationPacketListenerImpl.java:83-128`); each waits for the
/// client packet that finishes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ConfigurationTask {
    SynchronizeRegistries,
    CodeOfConduct,
    ResourcePack,
    JoinWorld,
}

struct ConfigurationState {
    profile: GameProfile,
    client_information: ClientInformation,
    client_brand: Option<String>,
    task: ConfigurationTask,
    pending_keep_alive: Option<i64>,
    reconfiguring: bool,
}

struct PlayState {
    profile: GameProfile,
    client_information: ClientInformation,
    client_brand: Option<String>,
    reconfigure_config: Option<Arc<ServerConnectionConfig>>,
}

enum SessionState {
    Handshake,
    Status { requested: bool },
    Login(Option<GameProfile>),
    Configuration(ConfigurationState),
    Play(PlayState),
}

pub(crate) struct ServerSession {
    transferred: bool,
    pub(crate) config: Arc<ServerConnectionConfig>,
    prepared: Arc<PreparedConnectionConfig>,
    state: SessionState,
    remote_address: Option<SocketAddr>,
}

impl ServerSession {
    pub(crate) fn new(
        config: Arc<ServerConnectionConfig>,
        remote_address: Option<SocketAddr>,
    ) -> Result<Self> {
        let prepared = Arc::clone(config.prepared()?);
        Ok(Self {
            config,
            prepared,
            state: SessionState::Handshake,
            transferred: false,
            remote_address,
        })
    }

    pub(crate) fn phase(&self) -> ConnectionPhase {
        match &self.state {
            SessionState::Handshake => ConnectionPhase::Handshake,
            SessionState::Status { .. } => ConnectionPhase::Status,
            SessionState::Login(_) => ConnectionPhase::Login,
            SessionState::Configuration(_) => ConnectionPhase::Configuration,
            SessionState::Play(_) => ConnectionPhase::Play,
        }
    }

    pub(crate) fn is_reconfiguring(&self) -> bool {
        match &self.state {
            SessionState::Play(play) => play.reconfigure_config.is_some(),
            SessionState::Configuration(configuration) => configuration.reconfiguring,
            _ => false,
        }
    }

    pub(crate) fn handle_packet(&mut self, packet_id: i32, payload: &[u8]) -> Result<SessionStep> {
        match &mut self.state {
            SessionState::Handshake => self.handle_handshake(packet_id, payload),
            SessionState::Status { requested } => {
                let packet = decode_status_serverbound(packet_id, payload)?;
                let mut step = SessionStep::default();
                match packet {
                    StatusServerbound::Request if !*requested => {
                        *requested = true;
                        step.cached_packet(&self.prepared.status_response);
                    }
                    StatusServerbound::Request => {
                        step.close = Some(ConnectionCloseReason::StatusHandled);
                    }
                    StatusServerbound::Ping { time } => {
                        step.packet(encode_status_pong(time));
                        step.close = Some(ConnectionCloseReason::StatusHandled);
                    }
                }
                Ok(step)
            }
            SessionState::Login(profile) => {
                let packet = decode_login_serverbound(packet_id, payload)?;
                match (profile, packet) {
                    (slot @ None, LoginServerbound::Hello { name, .. }) => {
                        let profile = GameProfile {
                            uuid: offline_player_uuid(&name),
                            name,
                            properties: Vec::new(),
                        };
                        let mut step = SessionStep::default();
                        // `verifyLoginAndFinishConnectionSetup` (`ServerLoginPacketListenerImpl.java:140-145`).
                        if let Some(reason) = self
                            .config
                            .admission
                            .as_ref()
                            .and_then(|admission| (admission.0)(&profile, self.remote_address))
                        {
                            step.packet(encode_login_disconnect_json(&nbt_summary_to_json_value(
                                reason.component(),
                            ))?);
                            step.close = Some(ConnectionCloseReason::LoginRefused);
                            return Ok(step);
                        }
                        if let Some((compression_packet, threshold)) =
                            &self.prepared.login_compression
                        {
                            step.outbound.push(OutboundPacket {
                                packet_id: compression_packet.0,
                                payload: compression_packet.1.clone(),
                                compression_after: Some(*threshold),
                            });
                        }
                        step.packet(encode_login_finished(&profile)?);
                        *slot = Some(profile);
                        Ok(step)
                    }
                    (slot @ Some(_), LoginServerbound::Acknowledged) => {
                        let profile = slot.take().expect("acknowledgement has a verified profile");
                        self.state = SessionState::Configuration(ConfigurationState {
                            profile,
                            client_information: ClientInformation::default(),
                            client_brand: None,
                            task: ConfigurationTask::SynchronizeRegistries,
                            pending_keep_alive: None,
                            reconfiguring: false,
                        });
                        let mut step = self.start_configuration();
                        step.transition(ConnectionPhase::Configuration);
                        Ok(step)
                    }
                    (_, LoginServerbound::Key(_)) => Err(protocol_violation(
                        "encryption key packet is invalid for offline login",
                    )),
                    (
                        _,
                        LoginServerbound::CustomQueryAnswer { .. }
                        | LoginServerbound::CookieResponse { .. },
                    ) => {
                        let mut step = SessionStep::default();
                        step.packet(encode_login_disconnect_json(&json!({
                            "translate": "multiplayer.disconnect.unexpected_query_response"
                        }))?);
                        step.close = Some(ConnectionCloseReason::UnexpectedQuery);
                        Ok(step)
                    }
                    _ => Err(protocol_violation("unexpected login packet order")),
                }
            }
            SessionState::Configuration(configuration) => {
                let packet = decode_configuration_serverbound(packet_id, payload)?;
                let mut step = SessionStep::default();
                match packet {
                    ConfigurationServerbound::ClientInformation(information) => {
                        configuration.client_information = information;
                    }
                    ConfigurationServerbound::CookieResponse { .. } => {
                        step.packet(encode_configuration_disconnect(&translatable(
                            "multiplayer.disconnect.unexpected_query_response",
                        ))?);
                        step.close = Some(ConnectionCloseReason::UnexpectedQuery);
                    }
                    ConfigurationServerbound::CustomPayload(payload) => {
                        if let CustomPayloadBody::Brand { brand } = payload.payload {
                            configuration.client_brand = Some(brand);
                        }
                    }
                    ConfigurationServerbound::SelectKnownPacks { known_packs }
                        if configuration.task == ConfigurationTask::SynchronizeRegistries =>
                    {
                        let use_vanilla_core = self.prepared.offers_vanilla_core
                            && known_packs.iter().any(is_vanilla_core_known_pack);
                        self.publish_synchronized_registries(use_vanilla_core, &mut step);
                        self.start_next_task(ConfigurationTask::SynchronizeRegistries, &mut step);
                    }
                    ConfigurationServerbound::AcceptCodeOfConduct
                        if configuration.task == ConfigurationTask::CodeOfConduct =>
                    {
                        self.start_next_task(ConfigurationTask::CodeOfConduct, &mut step);
                    }
                    // `ServerCommonPacketListenerImpl.handleResourcePackResponse`
                    // (`:105-111`), then the task finishes on a terminal action
                    // (`ServerConfigurationPacketListenerImpl.java:136-141`).
                    // Enforce `require-resource-pack` on all terminal failure
                    // or discard statuses so clients cannot bypass a required
                    // pack via download/reload errors.
                    ConfigurationServerbound::ResourcePack { action, .. } => {
                        if matches!(
                            action,
                            ResourcePackResponseAction::Declined
                                | ResourcePackResponseAction::FailedDownload
                                | ResourcePackResponseAction::InvalidUrl
                                | ResourcePackResponseAction::FailedReload
                                | ResourcePackResponseAction::Discarded
                        ) && self
                            .config
                            .resource_pack
                            .as_ref()
                            .is_some_and(|pack| pack.required)
                        {
                            step.packet(encode_configuration_disconnect(&translatable(
                                "multiplayer.requiredTexturePrompt.disconnect",
                            ))?);
                            step.close = Some(ConnectionCloseReason::ResourcePackDeclined);
                            return Ok(step);
                        }
                        if !matches!(
                            action,
                            ResourcePackResponseAction::Accepted
                                | ResourcePackResponseAction::Downloaded
                        ) {
                            if configuration.task != ConfigurationTask::ResourcePack {
                                return Err(protocol_violation(
                                    "resource pack response outside its configuration task",
                                ));
                            }
                            self.start_next_task(ConfigurationTask::ResourcePack, &mut step);
                        }
                    }
                    ConfigurationServerbound::Finish
                        if configuration.task == ConfigurationTask::JoinWorld =>
                    {
                        let profile = configuration.profile.clone();
                        let client_information = configuration.client_information.clone();
                        let client_brand = configuration.client_brand.clone();
                        let ready = ConnectionEvent::PlayReady {
                            transferred: self.transferred,
                            profile: profile.clone(),
                            client_information: client_information.clone(),
                            client_brand: client_brand.clone(),
                        };
                        self.state = SessionState::Play(PlayState {
                            profile,
                            client_information,
                            client_brand,
                            reconfigure_config: None,
                        });
                        step.transition(ConnectionPhase::Play);
                        step.events.push(ready);
                    }
                    // `ServerCommonPacketListenerImpl.handleKeepAlive`
                    // (`ServerCommonPacketListenerImpl.java:80-88`).
                    ConfigurationServerbound::KeepAlive { id } => {
                        if configuration.pending_keep_alive == Some(id) {
                            configuration.pending_keep_alive = None;
                        } else {
                            step.packet(encode_configuration_disconnect(&translatable(
                                "disconnect.timeout",
                            ))?);
                            step.close = Some(ConnectionCloseReason::KeepAliveTimeout);
                        }
                    }
                    ConfigurationServerbound::Pong { .. } => {}
                    // `ServerCommonPacketListenerImpl.handleCustomClickAction`
                    // (`ServerCommonPacketListenerImpl.java:99-102`).
                    ConfigurationServerbound::CustomClickAction(action) => {
                        step.events.push(ConnectionEvent::CustomClickAction(action));
                    }
                    _ => return Err(protocol_violation("unexpected configuration packet order")),
                }
                Ok(step)
            }
            SessionState::Play(play) => {
                if packet_id == bbb_protocol::ids::play::SERVERBOUND_CONFIGURATION_ACKNOWLEDGED
                    && payload.is_empty()
                {
                    if let Some(next_config) = play.reconfigure_config.take() {
                        let prepared = Arc::clone(next_config.prepared()?);
                        let profile = play.profile.clone();
                        let client_information = play.client_information.clone();
                        let client_brand = play.client_brand.clone();
                        self.config = next_config;
                        self.prepared = prepared;
                        self.state = SessionState::Configuration(ConfigurationState {
                            profile,
                            client_information,
                            client_brand,
                            task: ConfigurationTask::SynchronizeRegistries,
                            pending_keep_alive: None,
                            reconfiguring: true,
                        });
                        let mut step = self.start_configuration();
                        step.transition(ConnectionPhase::Configuration);
                        return Ok(step);
                    }
                }
                Ok(SessionStep {
                    events: vec![ConnectionEvent::PlayPacket {
                        packet_id,
                        payload: payload.to_vec(),
                    }],
                    ..SessionStep::default()
                })
            }
        }
    }

    /// `ServerGamePacketListenerImpl.switchToConfig`
    /// (`ServerGamePacketListenerImpl.java:2115-2122`).
    pub(crate) fn begin_reconfiguration(
        &mut self,
        config: Option<Arc<ServerConnectionConfig>>,
    ) -> Result<SessionStep> {
        let SessionState::Play(play) = &mut self.state else {
            return Err(protocol_violation(
                "server attempted to start configuration outside Play phase",
            ));
        };
        if play.reconfigure_config.is_some() {
            return Err(protocol_violation(
                "configuration switch is already pending acknowledgement",
            ));
        }
        let next_config = config.unwrap_or_else(|| Arc::clone(&self.config));
        let _ = next_config.prepared()?;
        play.reconfigure_config = Some(next_config);
        let mut step = SessionStep::default();
        step.packet(bbb_protocol::packets::encode_play_clientbound_start_configuration());
        Ok(step)
    }

    pub(crate) fn login_timeout() -> Result<SessionStep> {
        let mut step = SessionStep::default();
        step.packet(encode_login_disconnect_json(&json!({
            "translate": "multiplayer.disconnect.slow_login"
        }))?);
        step.close = Some(ConnectionCloseReason::LoginTimeout);
        Ok(step)
    }

    /// `ServerCommonPacketListenerImpl.keepConnectionAlive`
    /// (`ServerCommonPacketListenerImpl.java:118-133`).
    pub(crate) fn configuration_keep_alive(&mut self, challenge_id: i64) -> Result<SessionStep> {
        let SessionState::Configuration(configuration) = &mut self.state else {
            return Ok(SessionStep::default());
        };
        let mut step = SessionStep::default();
        if configuration.pending_keep_alive.is_some() {
            step.packet(encode_configuration_disconnect(&translatable(
                "disconnect.timeout",
            ))?);
            step.close = Some(ConnectionCloseReason::KeepAliveTimeout);
        } else {
            configuration.pending_keep_alive = Some(challenge_id);
            step.packet(encode_configuration_keep_alive_clientbound(challenge_id));
        }
        Ok(step)
    }

    fn handle_handshake(&mut self, packet_id: i32, payload: &[u8]) -> Result<SessionStep> {
        let intention = decode_client_intention(packet_id, payload)?;
        let mut step = SessionStep::default();
        match intention.intent {
            ClientIntent::Status if self.config.status_replies => {
                self.state = SessionState::Status { requested: false };
                step.transition(ConnectionPhase::Status);
            }
            // Vanilla closes without a reason packet: the status protocol has no
            // disconnect, so a refused probe simply ends.
            ClientIntent::Status => step.close = Some(ConnectionCloseReason::StatusIgnored),
            ClientIntent::Transfer if !self.config.accepts_transfers => {
                step.packet(encode_login_disconnect_json(&json!({
                    "translate": "multiplayer.disconnect.transfers_disabled"
                }))?);
                step.close = Some(ConnectionCloseReason::ProtocolViolation(
                    "transfer intention is disabled".to_string(),
                ));
            }
            ClientIntent::Login | ClientIntent::Transfer
                if intention.protocol_version == PROTOCOL_VERSION =>
            {
                self.transferred = intention.intent == ClientIntent::Transfer;
                self.state = SessionState::Login(None);
                step.transition(ConnectionPhase::Login);
            }
            ClientIntent::Login | ClientIntent::Transfer => {
                let translation = if intention.protocol_version < 754 {
                    "multiplayer.disconnect.outdated_client"
                } else {
                    "multiplayer.disconnect.incompatible"
                };
                step.packet(encode_login_disconnect_json(&json!({
                    "translate": translation,
                    "with": [bbb_protocol::MC_VERSION]
                }))?);
                step.close = Some(ConnectionCloseReason::ProtocolViolation(format!(
                    "client protocol {} does not match {}",
                    intention.protocol_version, PROTOCOL_VERSION
                )));
            }
        }
        Ok(step)
    }

    fn start_configuration(&self) -> SessionStep {
        let mut step = SessionStep::default();
        step.cached_packet(&self.prepared.brand);
        if let Some(links) = &self.prepared.server_links {
            step.cached_packet(links);
        }
        step.cached_packet(&self.prepared.enabled_features);
        step.cached_packet(&self.prepared.select_known_packs);
        step
    }

    /// Pack every synchronized registry, then tags, then FinishConfiguration
    /// (`SynchronizeRegistriesTask.java:35-44`).
    fn publish_synchronized_registries(&self, use_vanilla_core: bool, step: &mut SessionStep) {
        if let Some(content_registry) = &self.prepared.content_registry_packet {
            step.cached_packet(content_registry);
        }
        let frames = if use_vanilla_core {
            &self.prepared.registry_sync.core_dedup_frames
        } else {
            &self.prepared.registry_sync.full_frames
        };
        for frame in frames.iter() {
            step.cached_packet(frame);
        }
        step.cached_packet(&self.prepared.registry_sync.update_tags);
    }

    /// `finishCurrentTask` then `startNextTask`, skipping the optional tasks
    /// this server has nothing for (`addOptionalTasks`, `:110-128`).
    fn start_next_task(&mut self, finished: ConfigurationTask, step: &mut SessionStep) {
        let SessionState::Configuration(configuration) = &mut self.state else {
            unreachable!("configuration tasks run in the configuration phase");
        };
        let code_of_conduct = &self.prepared.code_of_conduct;
        let next = if finished < ConfigurationTask::CodeOfConduct && !code_of_conduct.is_empty() {
            let language = configuration.client_information.language.to_lowercase();
            let packet = code_of_conduct
                .get(&language)
                .or_else(|| code_of_conduct.get("en_us"))
                .or_else(|| code_of_conduct.values().next())
                .expect("the map is not empty");
            step.cached_packet(packet);
            ConfigurationTask::CodeOfConduct
        } else if let Some(pack_packet) = (finished < ConfigurationTask::ResourcePack)
            .then_some(self.prepared.resource_pack_push.as_ref())
            .flatten()
        {
            step.cached_packet(pack_packet);
            ConfigurationTask::ResourcePack
        } else {
            step.cached_packet(&self.prepared.configuration_finish);
            ConfigurationTask::JoinWorld
        };
        configuration.task = next;
    }
}

fn translatable(key: &str) -> RichTextComponent {
    RichTextComponent::try_from_component_nbt(
        component_translatable_nbt(key, Vec::new(), &ComponentStyle::default())
            .expect("a bare key has no style to project"),
    )
    .expect("a translatable key is a valid component")
}

fn protocol_violation(message: impl Into<String>) -> ServerNetError {
    ProtocolError::InvalidData(message.into()).into()
}

#[cfg(test)]
pub(crate) fn shared_registry_sync_ptr(config: &ServerConnectionConfig) -> Result<usize> {
    Ok(Arc::as_ptr(&config.prepared()?.registry_sync) as usize)
}
