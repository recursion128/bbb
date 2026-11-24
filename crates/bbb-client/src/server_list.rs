//! Vanilla-compatible `servers.dat` persistence for the optional startup
//! server-list owner.
//!
//! Vanilla reaches a server from the command line through
//! `QuickPlay.joinMultiplayerWorld`, which loads `servers.dat`, reuses or
//! creates the entry for the address, and hands that `ServerData` to the
//! connection — which is why a command-line join does persist
//! `ClientboundServerDataPacket` through `ServerList.saveSingleServer`. BBB
//! mirrors that owner but keeps it opt-in, so a run only touches a real game
//! directory when both `--server-list-store` and `--server-list-name` say to:
//! the remaining `ServerList` methods exist solely for the multiplayer
//! server-selection screen, which BBB does not have.

#[cfg(any(not(target_arch = "wasm32"), test))]
use std::{
    fs,
    io::{ErrorKind, Read},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    thread,
};

#[cfg(any(not(target_arch = "wasm32"), test))]
use anyhow::Context;
use anyhow::{bail, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use bbb_client_world::WorldStore;
#[cfg(test)]
use bbb_protocol::decode_named_nbt_summary;
use bbb_protocol::{
    decode_file_nbt_summary, encode_named_nbt_summary, NbtSummaryEntry, NbtSummaryValue,
};

const MAX_SERVER_LIST_FILE_BYTES: usize = 64 * 1024 * 1024;
/// `ServerList.MAX_HIDDEN_SERVERS`.
const MAX_HIDDEN_SERVERS: usize = 16;
#[cfg(any(not(target_arch = "wasm32"), test))]
static NEXT_TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(1);

pub(crate) struct ServerListPersistence {
    #[cfg(any(not(target_arch = "wasm32"), test))]
    io_tx: Option<mpsc::Sender<ServerListIoCommand>>,
    #[cfg(any(not(target_arch = "wasm32"), test))]
    io_thread: Option<thread::JoinHandle<()>>,
    #[cfg(target_arch = "wasm32")]
    current_name: String,
    #[cfg(target_arch = "wasm32")]
    current_address: String,
    pack_preference: Option<bool>,
    observed_server_data_packets: usize,
}

#[cfg(any(not(target_arch = "wasm32"), test))]
enum ServerListIoCommand {
    Save {
        icon_bytes: Option<Vec<u8>>,
        pack_preference: Option<bool>,
    },
    #[cfg(test)]
    Flush(mpsc::Sender<()>),
}

impl ServerListPersistence {
    #[cfg(any(not(target_arch = "wasm32"), test))]
    pub(crate) fn new(
        path: impl Into<PathBuf>,
        current_name: impl Into<String>,
        current_address: impl Into<String>,
        restore_preference: bool,
    ) -> Result<Self> {
        let path = path.into();
        let current_name = current_name.into();
        let current_address = current_address.into();
        let pack_preference = if restore_preference {
            match ServerListFile::load(&path) {
                Ok(file) => file.pack_preference(&current_address),
                Err(error) => {
                    tracing::warn!(%error, "unable to read the server resource-pack preference");
                    None
                }
            }
        } else {
            None
        };
        let (io_tx, io_rx) = mpsc::channel();
        let io_thread = thread::Builder::new()
            .name("bbb-server-list-io".to_string())
            .spawn(move || {
                while let Ok(command) = io_rx.recv() {
                    match command {
                        ServerListIoCommand::Save {
                            icon_bytes,
                            pack_preference,
                        } => {
                            if let Err(err) = save_server_entry(
                                &path,
                                &current_name,
                                &current_address,
                                icon_bytes.as_deref(),
                                pack_preference,
                            ) {
                                tracing::error!(?err, "failed to persist server-data update");
                            }
                        }
                        #[cfg(test)]
                        ServerListIoCommand::Flush(done) => {
                            let _ = done.send(());
                        }
                    }
                }
            })
            .context("spawn server-list IO worker")?;
        Ok(Self {
            io_tx: Some(io_tx),
            io_thread: Some(io_thread),
            pack_preference,
            observed_server_data_packets: 0,
        })
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn new_web(
        current_name: impl Into<String>,
        current_address: impl Into<String>,
        restore_preference: bool,
    ) -> Self {
        let current_name = current_name.into();
        let current_address = current_address.into();
        let pack_preference =
            match save_web_server_entry(&current_name, &current_address, None, None) {
                Ok(preference) if restore_preference => preference,
                Ok(_) => None,
                Err(error) => {
                    tracing::warn!(%error, "unable to open the browser server list");
                    None
                }
            };
        Self {
            current_name,
            current_address,
            pack_preference,
            observed_server_data_packets: 0,
        }
    }

    /// Mirrors `ClientPacketListener.handleServerData` followed by
    /// `ServerList.saveSingleServer`. MOTD remains session-only because
    /// `ServerData.write` does not serialize it; a validated icon updates the
    /// matching entry. Startup configuration creates the entry when absent,
    /// taking the place of Vanilla's add-server screen.
    pub(crate) fn persist_current_server_data(&mut self, world: &WorldStore) -> Result<bool> {
        let packet_count = world.counters().server_data_packets;
        if packet_count <= self.observed_server_data_packets {
            return Ok(false);
        }
        self.observed_server_data_packets = packet_count;

        let Some(server_data) = world.server_data() else {
            return Ok(false);
        };
        self.save_entry(server_data.icon_bytes.as_deref(), None)?;
        Ok(true)
    }

    pub(crate) fn pack_preference(&self) -> Option<bool> {
        self.pack_preference
    }

    pub(crate) fn persist_pack_preference(&mut self, preference: bool) -> Result<()> {
        if self.pack_preference != Some(preference) {
            self.pack_preference = Some(preference);
            self.save_entry(None, Some(preference))?;
        }
        Ok(())
    }

    fn save_entry(&self, icon_bytes: Option<&[u8]>, pack_preference: Option<bool>) -> Result<()> {
        #[cfg(target_arch = "wasm32")]
        save_web_server_entry(
            &self.current_name,
            &self.current_address,
            icon_bytes,
            pack_preference,
        )?;
        #[cfg(not(target_arch = "wasm32"))]
        self.io_tx
            .as_ref()
            .context("server-list IO worker is closed")?
            .send(ServerListIoCommand::Save {
                icon_bytes: icon_bytes.map(<[u8]>::to_vec),
                pack_preference,
            })
            .map_err(|_| anyhow::anyhow!("server-list IO worker stopped"))?;
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn flush(&self) -> Result<()> {
        let (done_tx, done_rx) = mpsc::channel();
        self.io_tx
            .as_ref()
            .context("server-list IO worker is closed")?
            .send(ServerListIoCommand::Flush(done_tx))
            .map_err(|_| anyhow::anyhow!("server-list IO worker stopped"))?;
        done_rx
            .recv()
            .map_err(|_| anyhow::anyhow!("server-list IO worker stopped before flush"))
    }
}

#[cfg(any(not(target_arch = "wasm32"), test))]
impl Drop for ServerListPersistence {
    fn drop(&mut self) {
        drop(self.io_tx.take());
        if let Some(worker) = self.io_thread.take() {
            if worker.join().is_err() {
                tracing::error!("server-list IO worker panicked during shutdown");
            }
        }
    }
}

#[cfg(any(not(target_arch = "wasm32"), test))]
fn save_server_entry(
    path: &Path,
    current_name: &str,
    current_address: &str,
    icon_bytes: Option<&[u8]>,
    pack_preference: Option<bool>,
) -> Result<()> {
    let mut file = ServerListFile::load(path)?;
    file.upsert_entry(current_name, current_address, icon_bytes, pack_preference)?;
    file.save(path)
}

#[cfg(target_arch = "wasm32")]
const WEB_SERVERS_STORAGE_KEY: &str = "bbb.servers.dat";

#[cfg(target_arch = "wasm32")]
pub(crate) fn most_recent_saved_web_server_address() -> Option<String> {
    let raw_b64 = web_sys::window()
        .and_then(|window| window.local_storage().ok().flatten())
        .and_then(|storage| storage.get_item(WEB_SERVERS_STORAGE_KEY).ok().flatten())?;
    let bytes = STANDARD.decode(raw_b64.trim()).ok()?;
    let file = ServerListFile::from_nbt_bytes(&bytes).ok()?;
    file.first_server_address()
}

#[cfg(target_arch = "wasm32")]
fn save_web_server_entry(
    current_name: &str,
    current_address: &str,
    icon_bytes: Option<&[u8]>,
    pack_preference: Option<bool>,
) -> Result<Option<bool>> {
    let Some(storage) = web_sys::window().and_then(|window| window.local_storage().ok().flatten())
    else {
        return Ok(None);
    };
    let mut file = match storage
        .get_item(WEB_SERVERS_STORAGE_KEY)
        .map_err(|err| anyhow::anyhow!("read browser server list: {err:?}"))?
    {
        Some(raw) => ServerListFile::from_nbt_bytes(&STANDARD.decode(raw.trim())?)?,
        None => ServerListFile::empty(),
    };
    let preference =
        file.upsert_entry(current_name, current_address, icon_bytes, pack_preference)?;
    storage
        .set_item(
            WEB_SERVERS_STORAGE_KEY,
            &STANDARD.encode(file.to_nbt_bytes()?),
        )
        .map_err(|err| anyhow::anyhow!("save browser server list: {err:?}"))?;
    Ok(preference)
}

struct ServerListFile {
    root: NbtSummaryValue,
}

impl ServerListFile {
    fn from_nbt_bytes(raw: &[u8]) -> Result<Self> {
        if raw.len() > MAX_SERVER_LIST_FILE_BYTES {
            bail!(
                "server list is {} bytes, exceeding the {} byte limit",
                raw.len(),
                MAX_SERVER_LIST_FILE_BYTES
            );
        }
        let Some(root) = decode_file_nbt_summary(raw)? else {
            bail!("server list has an end-tag root");
        };
        if !matches!(root, NbtSummaryValue::Compound(_)) {
            bail!("server list root is not a compound");
        }
        Ok(Self { root })
    }

    fn to_nbt_bytes(&self) -> Result<Vec<u8>> {
        Ok(encode_named_nbt_summary(
            "",
            &self.root,
            MAX_SERVER_LIST_FILE_BYTES,
        )?)
    }

    fn servers(&self) -> Option<&[NbtSummaryValue]> {
        let NbtSummaryValue::Compound(root) = &self.root else {
            unreachable!("server-list root shape checked while loading");
        };
        match &root.iter().find(|e| e.name == "servers")?.value {
            NbtSummaryValue::List(servers) => Some(servers),
            _ => None,
        }
    }

    #[cfg(any(not(target_arch = "wasm32"), test))]
    fn pack_preference(&self, address: &str) -> Option<bool> {
        let servers = self.servers()?;
        let index = server_index(servers, address)?;
        let NbtSummaryValue::Compound(entry) = &servers[index] else {
            unreachable!("matching server entry is a compound");
        };
        entry_pack_preference(entry)
    }

    #[cfg(any(target_arch = "wasm32", test))]
    fn first_server_address(&self) -> Option<String> {
        let servers = self.servers()?;
        servers.iter().find_map(|entry| match entry {
            NbtSummaryValue::Compound(fields) => compound_string(fields, "ip"),
            _ => None,
        })
    }

    fn upsert_entry(
        &mut self,
        current_name: &str,
        current_address: &str,
        icon_bytes: Option<&[u8]>,
        pack_preference: Option<bool>,
    ) -> Result<Option<bool>> {
        let entry = self.find_or_insert(current_name, current_address)?;
        if let Some(icon) = icon_bytes {
            let encoded = STANDARD.encode(icon);
            // Vanilla's StringFallbackDataOutput writes an empty NBT string when
            // DataOutput.writeUTF rejects an oversized base64 value.
            let encoded = if encoded.len() > usize::from(u16::MAX) {
                String::new()
            } else {
                encoded
            };
            set_compound_field(entry, "icon", NbtSummaryValue::string(encoded));
        }
        if let Some(preference) = pack_preference {
            set_compound_field(
                entry,
                "acceptTextures",
                NbtSummaryValue::Byte(i8::from(preference)),
            );
        }
        Ok(entry_pack_preference(entry))
    }

    #[cfg(any(not(target_arch = "wasm32"), test))]
    fn load(path: &Path) -> Result<Self> {
        let file = match fs::File::open(path) {
            Ok(file) => file,
            Err(err) if err.kind() == ErrorKind::NotFound => {
                return Ok(Self::empty());
            }
            Err(err) => {
                return Err(err).with_context(|| format!("read server list {}", path.display()));
            }
        };
        let mut raw = Vec::new();
        file.take((MAX_SERVER_LIST_FILE_BYTES as u64) + 1)
            .read_to_end(&mut raw)
            .with_context(|| format!("read server list {}", path.display()))?;
        if raw.len() > MAX_SERVER_LIST_FILE_BYTES {
            bail!(
                "server list {} is {} bytes, exceeding the {} byte limit",
                path.display(),
                raw.len(),
                MAX_SERVER_LIST_FILE_BYTES
            );
        }
        Self::from_nbt_bytes(&raw).with_context(|| format!("decode server list {}", path.display()))
    }

    fn empty() -> Self {
        Self {
            root: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                name: "servers".into(),
                value: NbtSummaryValue::List(Vec::new()),
            }]),
        }
    }

    /// Vanilla `ServerList.get(String)` followed, on a miss, by
    /// `QuickPlay.joinMultiplayerWorld`'s `add(data, true)`.
    ///
    /// The lookup compares the address alone and scans the visible list before
    /// the hidden one, so an entry the player already named for this address is
    /// reused instead of duplicated; the configured name only ever names an
    /// entry this owner creates, exactly as `selectServer.defaultName` does.
    /// A created entry is hidden, which is what keeps a command-line join out
    /// of the server list the player curates by hand.
    fn find_or_insert(&mut self, name: &str, address: &str) -> Result<&mut Vec<NbtSummaryEntry>> {
        let NbtSummaryValue::Compound(root) = &mut self.root else {
            unreachable!("server-list root shape checked while loading");
        };
        let servers = compound_field_mut(root, "servers");
        if !matches!(servers, Some(NbtSummaryValue::List(_))) {
            set_compound_field(root, "servers", NbtSummaryValue::List(Vec::new()));
        }
        let Some(NbtSummaryValue::List(servers)) = compound_field_mut(root, "servers") else {
            unreachable!("servers field was normalized to a list");
        };
        if let Some(index) = server_index(servers, address) {
            let NbtSummaryValue::Compound(entry) = &mut servers[index] else {
                unreachable!("matching server entry is a compound");
            };
            return Ok(entry);
        }

        // `add` puts a hidden entry at the head of the hidden list, and `save`
        // writes the visible entries before it, so the hidden block starts at
        // the first flagged entry.
        let head = servers
            .iter()
            .position(|server| matches!(server, NbtSummaryValue::Compound(entries) if entry_is_hidden(entries)))
            .unwrap_or(servers.len());
        servers.insert(
            head,
            NbtSummaryValue::Compound(vec![
                NbtSummaryEntry {
                    name: "name".into(),
                    value: NbtSummaryValue::string(name),
                },
                NbtSummaryEntry {
                    name: "ip".into(),
                    value: NbtSummaryValue::string(address),
                },
                NbtSummaryEntry {
                    name: "hidden".into(),
                    value: NbtSummaryValue::Byte(1),
                },
            ]),
        );
        // `add` then trims the tail of the hidden list; every entry it drops
        // sits after the one just inserted, so `head` still addresses it.
        let mut hidden_seen = 0usize;
        servers.retain(|server| {
            let hidden =
                matches!(server, NbtSummaryValue::Compound(entries) if entry_is_hidden(entries));
            if !hidden {
                return true;
            }
            hidden_seen += 1;
            hidden_seen <= MAX_HIDDEN_SERVERS
        });
        let Some(NbtSummaryValue::Compound(entry)) = servers.get_mut(head) else {
            unreachable!("new server entry is a compound");
        };
        Ok(entry)
    }

    #[cfg(any(not(target_arch = "wasm32"), test))]
    fn save(&self, path: &Path) -> Result<()> {
        let raw = self
            .to_nbt_bytes()
            .with_context(|| format!("encode server list {}", path.display()))?;
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent)
                .with_context(|| format!("create server list directory {}", parent.display()))?;
        }
        safely_replace(path, &raw)
    }
}

fn entry_pack_preference(entries: &[NbtSummaryEntry]) -> Option<bool> {
    match entries
        .iter()
        .find(|entry| entry.name == "acceptTextures")?
        .value
    {
        NbtSummaryValue::Byte(value) => Some(value != 0),
        _ => None,
    }
}

fn server_index(servers: &[NbtSummaryValue], address: &str) -> Option<usize> {
    servers
        .iter()
        .enumerate()
        .filter_map(|(index, server)| {
            let NbtSummaryValue::Compound(entries) = server else {
                return None;
            };
            (compound_string(entries, "ip").as_deref() == Some(address))
                .then(|| (entry_is_hidden(entries), index))
        })
        .min()
        .map(|(_, index)| index)
}

fn compound_field_mut<'a>(
    entries: &'a mut [NbtSummaryEntry],
    name: &str,
) -> Option<&'a mut NbtSummaryValue> {
    entries
        .iter_mut()
        .find(|entry| entry.name == name)
        .map(|entry| &mut entry.value)
}

fn compound_string(entries: &[NbtSummaryEntry], name: &str) -> Option<String> {
    entries.iter().find_map(|entry| {
        (entry.name == name)
            .then_some(&entry.value)
            .and_then(|value| match value {
                NbtSummaryValue::String(value) => value.try_to_string().ok(),
                _ => None,
            })
    })
}

/// `ServerList.load` splits the file into a visible and a hidden list on this
/// flag, reading it with `getBooleanOr("hidden", false)`; BBB keeps the file's
/// own order and only asks the same question of each entry.
fn entry_is_hidden(entries: &[NbtSummaryEntry]) -> bool {
    entries.iter().any(|entry| {
        entry.name == "hidden" && matches!(entry.value, NbtSummaryValue::Byte(flag) if flag != 0)
    })
}

fn set_compound_field(entries: &mut Vec<NbtSummaryEntry>, name: &str, value: NbtSummaryValue) {
    if let Some(entry) = entries.iter_mut().find(|entry| entry.name == name) {
        entry.value = value;
    } else {
        entries.push(NbtSummaryEntry {
            name: name.into(),
            value,
        });
    }
}

/// Vanilla `Util.safeReplaceFile`: write beside the destination, retain one
/// `_old` backup, then replace the live file. A failed final rename restores
/// the prior file.
#[cfg(any(not(target_arch = "wasm32"), test))]
fn safely_replace(path: &Path, raw: &[u8]) -> Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("servers.dat");
    let sequence = NEXT_TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    #[cfg(not(target_arch = "wasm32"))]
    let pid = std::process::id();
    #[cfg(target_arch = "wasm32")]
    let pid = 0_u32;
    let temp = parent.join(format!(".{file_name}.bbb-tmp-{pid}-{sequence}"));
    let backup = parent.join(format!("{file_name}_old"));
    fs::write(&temp, raw)
        .with_context(|| format!("write temporary server list {}", temp.display()))?;

    if backup.exists() {
        fs::remove_file(&backup)
            .with_context(|| format!("remove old server list backup {}", backup.display()))?;
    }
    let had_current = path.exists();
    if had_current {
        fs::rename(path, &backup).with_context(|| {
            format!(
                "move current server list {} to {}",
                path.display(),
                backup.display()
            )
        })?;
    }
    if let Err(err) = fs::rename(&temp, path) {
        if had_current {
            let _ = fs::rename(&backup, path);
        }
        let _ = fs::remove_file(&temp);
        return Err(err).with_context(|| format!("replace server list {}", path.display()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::{packets::ServerData, RichTextComponent};
    use std::{
        env, process,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn unique_store_path(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!(
            "bbb-server-list-{name}-{}-{nanos}.dat",
            process::id()
        ))
    }

    fn png_icon(width: i32, height: i32) -> Vec<u8> {
        let mut bytes = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        bytes.extend_from_slice(&13i32.to_be_bytes());
        bytes.extend_from_slice(b"IHDR");
        bytes.extend_from_slice(&width.to_be_bytes());
        bytes.extend_from_slice(&height.to_be_bytes());
        bytes
    }

    fn decoded_servers(path: &Path) -> Vec<NbtSummaryValue> {
        let (_, NbtSummaryValue::Compound(root)) =
            decode_named_nbt_summary(&fs::read(path).unwrap())
                .unwrap()
                .unwrap()
        else {
            panic!("server-list compound root");
        };
        let NbtSummaryValue::List(servers) = root
            .into_iter()
            .find(|entry| entry.name == "servers")
            .expect("servers field")
            .value
        else {
            panic!("servers list");
        };
        servers
    }

    #[test]
    fn server_list_ignores_raw_root_name_and_saves_empty_name() {
        let path = unique_store_path("raw-root");
        ServerListFile::empty().save(&path).unwrap();
        let mut raw = fs::read(&path).unwrap();
        raw.splice(1..3, [0, 3, 0xff, 0xff, 0xff]);
        fs::write(&path, &raw).unwrap();
        assert!(decode_named_nbt_summary(&raw).is_err());
        let file = ServerListFile::load(&path).unwrap();
        file.save(&path).unwrap();
        assert_eq!(
            decode_named_nbt_summary(&fs::read(&path).unwrap())
                .unwrap()
                .unwrap()
                .0,
            ""
        );
        let _ = fs::remove_file(path);
    }

    #[test]
    fn configured_owner_persists_valid_icon_in_vanilla_servers_dat_shape() {
        let path = unique_store_path("valid");
        let icon = png_icon(64, 64);
        let mut world = WorldStore::new();
        world.apply_server_data(ServerData {
            motd: RichTextComponent::plain("Packet MOTD"),
            icon_bytes: Some(icon.clone()),
        });

        let mut persistence =
            ServerListPersistence::new(&path, "Native Server", "example.org:25565", true).unwrap();
        assert!(persistence.persist_current_server_data(&world).unwrap());
        assert!(!persistence.persist_current_server_data(&world).unwrap());
        drop(persistence);

        let servers = decoded_servers(&path);
        let NbtSummaryValue::Compound(entry) = &servers[0] else {
            panic!("server compound");
        };
        assert_eq!(
            compound_string(entry, "name").as_deref(),
            Some("Native Server")
        );
        assert_eq!(
            compound_string(entry, "ip").as_deref(),
            Some("example.org:25565")
        );
        assert_eq!(
            compound_string(entry, "icon").as_deref(),
            Some(STANDARD.encode(icon).as_str())
        );
        assert!(compound_string(entry, "motd").is_none());
        // `QuickPlay.joinMultiplayerWorld` adds its entry hidden, so a
        // command-line join stays out of the hand-curated server list.
        assert!(matches!(
            compound_field(entry, "hidden"),
            Some(NbtSummaryValue::Byte(1))
        ));
        let _ = fs::remove_file(path);
    }

    fn named_server(name: &str, address: &str, hidden: bool) -> NbtSummaryValue {
        NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "name".into(),
                value: NbtSummaryValue::string(name),
            },
            NbtSummaryEntry {
                name: "ip".into(),
                value: NbtSummaryValue::string(address),
            },
            NbtSummaryEntry {
                name: "hidden".into(),
                value: NbtSummaryValue::Byte(i8::from(hidden)),
            },
        ])
    }

    fn write_servers(path: &Path, servers: Vec<NbtSummaryValue>) {
        let root = NbtSummaryValue::Compound(vec![NbtSummaryEntry {
            name: "servers".into(),
            value: NbtSummaryValue::List(servers),
        }]);
        fs::write(
            path,
            encode_named_nbt_summary("", &root, MAX_SERVER_LIST_FILE_BYTES).unwrap(),
        )
        .unwrap();
    }

    fn persist_icon(path: &Path, name: &str, address: &str) {
        let mut world = WorldStore::new();
        world.apply_server_data(ServerData {
            motd: RichTextComponent::plain("MOTD"),
            icon_bytes: Some(png_icon(64, 64)),
        });
        let mut persistence = ServerListPersistence::new(path, name, address, true).unwrap();
        assert!(persistence.persist_current_server_data(&world).unwrap());
        persistence.flush().unwrap();
    }

    #[test]
    fn an_entry_the_player_already_named_for_the_address_is_reused() {
        let path = unique_store_path("reuse");
        write_servers(
            &path,
            vec![
                named_server("Unrelated", "other.invalid", false),
                named_server("Player Named", "example.org:25565", false),
                named_server("Stale Hidden", "example.org:25565", true),
            ],
        );

        persist_icon(&path, "Native Server", "example.org:25565");

        // `ServerList.get(String)` compares the address alone and scans the
        // visible list first, so neither the configured name nor the hidden
        // duplicate wins.
        let servers = decoded_servers(&path);
        assert_eq!(servers.len(), 3);
        let NbtSummaryValue::Compound(entry) = &servers[1] else {
            panic!("server compound");
        };
        assert_eq!(
            compound_string(entry, "name").as_deref(),
            Some("Player Named")
        );
        assert!(compound_string(entry, "icon").is_some());
        let NbtSummaryValue::Compound(hidden) = &servers[2] else {
            panic!("server compound");
        };
        assert!(compound_string(hidden, "icon").is_none());
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_file_name(format!(
            "{}_old",
            path.file_name().unwrap().to_string_lossy()
        )));
    }

    #[test]
    fn a_created_entry_heads_the_hidden_block_and_trims_it_to_sixteen() {
        let path = unique_store_path("trim");
        let mut servers = vec![named_server("Curated", "curated.invalid", false)];
        servers.extend(
            (0..MAX_HIDDEN_SERVERS)
                .map(|index| named_server(&format!("Recent {index}"), &format!("h{index}"), true)),
        );
        write_servers(&path, servers);

        persist_icon(&path, "Native Server", "example.org:25565");

        let servers = decoded_servers(&path);
        // One visible entry plus a hidden block still capped at sixteen.
        assert_eq!(servers.len(), 1 + MAX_HIDDEN_SERVERS);
        let names: Vec<String> = servers
            .iter()
            .map(|server| {
                let NbtSummaryValue::Compound(entries) = server else {
                    panic!("server compound");
                };
                compound_string(entries, "name").expect("named entry")
            })
            .collect();
        assert_eq!(names[0], "Curated");
        assert_eq!(names[1], "Native Server");
        assert_eq!(names[2], "Recent 0");
        // `add` drops from the tail of the hidden list, so the oldest goes.
        assert_eq!(names[MAX_HIDDEN_SERVERS], "Recent 14");
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_file_name(format!(
            "{}_old",
            path.file_name().unwrap().to_string_lossy()
        )));
    }

    #[test]
    fn update_preserves_other_entries_unknown_fields_and_omitted_icon() {
        let path = unique_store_path("preserve");
        let old_icon = STANDARD.encode(png_icon(32, 32));
        let root = NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "version".into(),
                value: NbtSummaryValue::Int(7),
            },
            NbtSummaryEntry {
                name: "servers".into(),
                value: NbtSummaryValue::List(vec![
                    NbtSummaryValue::Compound(vec![
                        NbtSummaryEntry {
                            name: "name".into(),
                            value: NbtSummaryValue::string("Native Server"),
                        },
                        NbtSummaryEntry {
                            name: "ip".into(),
                            value: NbtSummaryValue::string("example.org:25565"),
                        },
                        NbtSummaryEntry {
                            name: "icon".into(),
                            value: NbtSummaryValue::string(old_icon.clone()),
                        },
                        NbtSummaryEntry {
                            name: "acceptedCodeOfConduct".into(),
                            value: NbtSummaryValue::Int(42),
                        },
                    ]),
                    NbtSummaryValue::Compound(vec![
                        NbtSummaryEntry {
                            name: "name".into(),
                            value: NbtSummaryValue::string("Other"),
                        },
                        NbtSummaryEntry {
                            name: "ip".into(),
                            value: NbtSummaryValue::string("other.invalid"),
                        },
                    ]),
                ]),
            },
        ]);
        fs::write(
            &path,
            encode_named_nbt_summary("", &root, MAX_SERVER_LIST_FILE_BYTES).unwrap(),
        )
        .unwrap();

        let mut world = WorldStore::new();
        world.apply_server_data(ServerData {
            motd: RichTextComponent::plain("No icon update"),
            icon_bytes: None,
        });
        let mut persistence =
            ServerListPersistence::new(&path, "Native Server", "example.org:25565", true).unwrap();
        assert!(persistence.persist_current_server_data(&world).unwrap());
        persistence.flush().unwrap();

        let servers = decoded_servers(&path);
        assert_eq!(servers.len(), 2);
        let NbtSummaryValue::Compound(entry) = &servers[0] else {
            panic!("server compound");
        };
        assert_eq!(
            compound_string(entry, "icon").as_deref(),
            Some(old_icon.as_str())
        );
        assert!(matches!(
            compound_field(entry, "acceptedCodeOfConduct"),
            Some(NbtSummaryValue::Int(42))
        ));
        let NbtSummaryValue::Compound(other) = &servers[1] else {
            panic!("other server compound");
        };
        assert_eq!(compound_string(other, "name").as_deref(), Some("Other"));
        let backup = path.with_file_name(format!(
            "{}_old",
            path.file_name().unwrap().to_string_lossy()
        ));
        assert!(backup.exists());
        let _ = fs::remove_file(path);
        let _ = fs::remove_file(backup);
    }

    #[test]
    fn server_pack_preference_survives_reconnect_and_prompts_for_a_required_pack() {
        use bbb_protocol::packets::ResourcePackPush;
        let path = unique_store_path("pack-preference");
        let address = "pack.example:25565";
        let mut store = ServerListPersistence::new(&path, "Pack Server", address, true).unwrap();
        assert_eq!(store.pack_preference(), None);
        store.persist_pack_preference(false).unwrap();
        drop(store);

        let mut store = ServerListPersistence::new(&path, "Pack Server", address, true).unwrap();
        assert_eq!(store.pack_preference(), Some(false));
        let mut world = WorldStore::new();
        world.set_resource_pack_preference(store.pack_preference());
        let id = uuid::Uuid::from_u128(1);
        let required_pack = ResourcePackPush {
            id,
            url: "https://example.org/required.zip".into(),
            hash: String::new(),
            required: true,
            prompt: None,
        };
        world.apply_resource_pack_push(required_pack.clone());
        world.request_resource_pack_consent(id);
        let sequence = world.resource_packs()[&id].push_sequence;
        assert!(world.answer_resource_pack_prompt(sequence, true));
        store
            .persist_pack_preference(world.resource_pack_preference().unwrap())
            .unwrap();
        drop(store);

        let saved = ServerListFile::load(&path).unwrap();
        assert_eq!(saved.pack_preference(address), Some(true));
        assert_eq!(saved.pack_preference("other.example:25565"), None);
        let explicit = ServerListPersistence::new(&path, "Pack Server", address, false).unwrap();
        assert_eq!(explicit.pack_preference(), None);
        world.begin_connection_session(false);
        world.apply_resource_pack_push(required_pack);
        world.request_resource_pack_consent(id);
        assert!(world.answer_resource_pack_prompt(world.resource_packs()[&id].push_sequence, false));
        assert_eq!(world.resource_pack_preference(), None);
        drop(explicit);
        fs::remove_file(path.with_file_name(format!(
            "{}_old",
            path.file_name().unwrap().to_string_lossy()
        )))
        .unwrap();
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn server_list_nbt_bytes_roundtrip_and_extract_first_server_address() {
        let mut file = ServerListFile::empty();
        file.upsert_entry("Web Gateway", "play.example.org:25570", Some(b"png"), None)
            .unwrap();
        let bytes = file.to_nbt_bytes().unwrap();
        let decoded = ServerListFile::from_nbt_bytes(&bytes).unwrap();
        assert_eq!(
            decoded.first_server_address().as_deref(),
            Some("play.example.org:25570")
        );
        assert!(ServerListFile::from_nbt_bytes(b"corrupt").is_err());
    }

    fn compound_field<'a>(
        entries: &'a [NbtSummaryEntry],
        name: &str,
    ) -> Option<&'a NbtSummaryValue> {
        entries
            .iter()
            .find(|entry| entry.name == name)
            .map(|entry| &entry.value)
    }
}
