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

use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use bbb_protocol::{
    decode_named_nbt_summary, encode_named_nbt_summary, NbtSummaryEntry, NbtSummaryValue,
};
use bbb_world::WorldStore;

const MAX_SERVER_LIST_FILE_BYTES: usize = 64 * 1024 * 1024;
/// `ServerList.MAX_HIDDEN_SERVERS`.
const MAX_HIDDEN_SERVERS: usize = 16;
static NEXT_TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(1);

pub(crate) struct ServerListPersistence {
    io_tx: Option<mpsc::Sender<ServerListIoCommand>>,
    io_thread: Option<thread::JoinHandle<()>>,
    observed_server_data_packets: usize,
}

enum ServerListIoCommand {
    SaveIcon {
        icon_bytes: Option<Vec<u8>>,
    },
    #[cfg(test)]
    Flush(mpsc::Sender<()>),
}

impl ServerListPersistence {
    pub(crate) fn new(
        path: impl Into<PathBuf>,
        current_name: impl Into<String>,
        current_address: impl Into<String>,
    ) -> Result<Self> {
        let path = path.into();
        let current_name = current_name.into();
        let current_address = current_address.into();
        let (io_tx, io_rx) = mpsc::channel();
        let io_thread = thread::Builder::new()
            .name("bbb-server-list-io".to_string())
            .spawn(move || {
                while let Ok(command) = io_rx.recv() {
                    match command {
                        ServerListIoCommand::SaveIcon { icon_bytes } => {
                            if let Err(err) = save_server_icon(
                                &path,
                                &current_name,
                                &current_address,
                                icon_bytes.as_deref(),
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
            observed_server_data_packets: 0,
        })
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
        self.io_tx
            .as_ref()
            .context("server-list IO worker is closed")?
            .send(ServerListIoCommand::SaveIcon {
                icon_bytes: server_data.icon_bytes.clone(),
            })
            .map_err(|_| anyhow::anyhow!("server-list IO worker stopped"))?;
        Ok(true)
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

fn save_server_icon(
    path: &Path,
    current_name: &str,
    current_address: &str,
    icon_bytes: Option<&[u8]>,
) -> Result<()> {
    let mut file = ServerListFile::load(path)?;
    let entry = file.find_or_insert(current_name, current_address)?;
    if let Some(icon) = icon_bytes {
        let encoded = STANDARD.encode(icon);
        // Vanilla's StringFallbackDataOutput writes an empty NBT string when
        // DataOutput.writeUTF rejects an oversized base64 value.
        let encoded = if encoded.len() > usize::from(u16::MAX) {
            String::new()
        } else {
            encoded
        };
        set_compound_field(entry, "icon", NbtSummaryValue::String(encoded));
    }
    file.save(path)
}

struct ServerListFile {
    root_name: String,
    root: NbtSummaryValue,
}

impl ServerListFile {
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
        let Some((root_name, root)) = decode_named_nbt_summary(&raw)
            .with_context(|| format!("decode server list {}", path.display()))?
        else {
            bail!("server list {} has an end-tag root", path.display());
        };
        if !matches!(root, NbtSummaryValue::Compound(_)) {
            bail!("server list {} root is not a compound", path.display());
        }
        Ok(Self { root_name, root })
    }

    fn empty() -> Self {
        Self {
            root_name: String::new(),
            root: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                name: "servers".to_string(),
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
        let existing = servers
            .iter()
            .enumerate()
            .filter_map(|(index, server)| {
                let NbtSummaryValue::Compound(entries) = server else {
                    return None;
                };
                (compound_string(entries, "ip") == Some(address))
                    .then(|| (entry_is_hidden(entries), index))
            })
            .min();
        if let Some((_, index)) = existing {
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
                    name: "name".to_string(),
                    value: NbtSummaryValue::String(name.to_string()),
                },
                NbtSummaryEntry {
                    name: "ip".to_string(),
                    value: NbtSummaryValue::String(address.to_string()),
                },
                NbtSummaryEntry {
                    name: "hidden".to_string(),
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

    fn save(&self, path: &Path) -> Result<()> {
        let raw = encode_named_nbt_summary(&self.root_name, &self.root, MAX_SERVER_LIST_FILE_BYTES)
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

fn compound_field_mut<'a>(
    entries: &'a mut [NbtSummaryEntry],
    name: &str,
) -> Option<&'a mut NbtSummaryValue> {
    entries
        .iter_mut()
        .find(|entry| entry.name == name)
        .map(|entry| &mut entry.value)
}

fn compound_string<'a>(entries: &'a [NbtSummaryEntry], name: &str) -> Option<&'a str> {
    entries.iter().find_map(|entry| {
        (entry.name == name)
            .then_some(&entry.value)
            .and_then(|value| match value {
                NbtSummaryValue::String(value) => Some(value.as_str()),
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
            name: name.to_string(),
            value,
        });
    }
}

/// Vanilla `Util.safeReplaceFile`: write beside the destination, retain one
/// `_old` backup, then replace the live file. A failed final rename restores
/// the prior file.
fn safely_replace(path: &Path, raw: &[u8]) -> Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("servers.dat");
    let sequence = NEXT_TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temp = parent.join(format!(
        ".{file_name}.bbb-tmp-{}-{sequence}",
        std::process::id()
    ));
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
    fn configured_owner_persists_valid_icon_in_vanilla_servers_dat_shape() {
        let path = unique_store_path("valid");
        let icon = png_icon(64, 64);
        let mut world = WorldStore::new();
        world.apply_server_data(ServerData {
            motd: RichTextComponent::plain("Packet MOTD"),
            icon_bytes: Some(icon.clone()),
        });

        let mut persistence =
            ServerListPersistence::new(&path, "Native Server", "example.org:25565").unwrap();
        assert!(persistence.persist_current_server_data(&world).unwrap());
        assert!(!persistence.persist_current_server_data(&world).unwrap());
        drop(persistence);

        let servers = decoded_servers(&path);
        let NbtSummaryValue::Compound(entry) = &servers[0] else {
            panic!("server compound");
        };
        assert_eq!(compound_string(entry, "name"), Some("Native Server"));
        assert_eq!(compound_string(entry, "ip"), Some("example.org:25565"));
        assert_eq!(
            compound_string(entry, "icon"),
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
                name: "name".to_string(),
                value: NbtSummaryValue::String(name.to_string()),
            },
            NbtSummaryEntry {
                name: "ip".to_string(),
                value: NbtSummaryValue::String(address.to_string()),
            },
            NbtSummaryEntry {
                name: "hidden".to_string(),
                value: NbtSummaryValue::Byte(i8::from(hidden)),
            },
        ])
    }

    fn write_servers(path: &Path, servers: Vec<NbtSummaryValue>) {
        let root = NbtSummaryValue::Compound(vec![NbtSummaryEntry {
            name: "servers".to_string(),
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
        let mut persistence = ServerListPersistence::new(path, name, address).unwrap();
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
        assert_eq!(compound_string(entry, "name"), Some("Player Named"));
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
        let names: Vec<&str> = servers
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
                name: "version".to_string(),
                value: NbtSummaryValue::Int(7),
            },
            NbtSummaryEntry {
                name: "servers".to_string(),
                value: NbtSummaryValue::List(vec![
                    NbtSummaryValue::Compound(vec![
                        NbtSummaryEntry {
                            name: "name".to_string(),
                            value: NbtSummaryValue::String("Native Server".to_string()),
                        },
                        NbtSummaryEntry {
                            name: "ip".to_string(),
                            value: NbtSummaryValue::String("example.org:25565".to_string()),
                        },
                        NbtSummaryEntry {
                            name: "icon".to_string(),
                            value: NbtSummaryValue::String(old_icon.clone()),
                        },
                        NbtSummaryEntry {
                            name: "acceptedCodeOfConduct".to_string(),
                            value: NbtSummaryValue::Int(42),
                        },
                    ]),
                    NbtSummaryValue::Compound(vec![
                        NbtSummaryEntry {
                            name: "name".to_string(),
                            value: NbtSummaryValue::String("Other".to_string()),
                        },
                        NbtSummaryEntry {
                            name: "ip".to_string(),
                            value: NbtSummaryValue::String("other.invalid".to_string()),
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
            ServerListPersistence::new(&path, "Native Server", "example.org:25565").unwrap();
        assert!(persistence.persist_current_server_data(&world).unwrap());
        persistence.flush().unwrap();

        let servers = decoded_servers(&path);
        assert_eq!(servers.len(), 2);
        let NbtSummaryValue::Compound(entry) = &servers[0] else {
            panic!("server compound");
        };
        assert_eq!(compound_string(entry, "icon"), Some(old_icon.as_str()));
        assert!(matches!(
            compound_field(entry, "acceptedCodeOfConduct"),
            Some(NbtSummaryValue::Int(42))
        ));
        let NbtSummaryValue::Compound(other) = &servers[1] else {
            panic!("other server compound");
        };
        assert_eq!(compound_string(other, "name"), Some("Other"));
        let backup = path.with_file_name(format!(
            "{}_old",
            path.file_name().unwrap().to_string_lossy()
        ));
        assert!(backup.exists());
        let _ = fs::remove_file(path);
        let _ = fs::remove_file(backup);
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
