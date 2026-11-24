use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use bbb_protocol::packets::{
    CustomPayload, CustomReportDetails, KnownPack, PlayClientbound, PlayerPositionUpdate,
    RegistryData, ResourcePackPop, ResourcePackPush, ResourcePackResponseAction, ServerLinks,
    ShowDialog, Transfer, UpdateEnabledFeatures, UpdateTags,
};
use bbb_protocol::{ComponentStyle, StyledTextRun};
use serde::{Deserialize, Serialize};

use super::ConnectionState;

pub const MAX_DISCONNECT_REASON_RETAINED_BYTES: usize = 65_536;
pub const MAX_DISCONNECT_REASON_RUNS: usize = 1_024;
const MAX_DISCONNECT_REASON_FONT_BYTES: usize = 256;

/// Bounded, presentation-safe subset of one terminal Component run.
///
/// `MultiLineTextWidget` renders style but does not expose Component click or
/// hover payloads, so those potentially deep payloads are discarded at the
/// net boundary rather than retained by the terminal screen generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisconnectTextRun {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub underlined: bool,
    pub strikethrough: bool,
    pub obfuscated: bool,
    pub color: Option<u32>,
    #[serde(default)]
    pub shadow_color: Option<u32>,
    pub font: Option<String>,
}

/// Bounded terminal reason delivered after all preceding connection events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisconnectReason {
    pub runs: Vec<DisconnectTextRun>,
    pub source_runs: usize,
    pub source_text_bytes: usize,
    pub retained_bytes: usize,
    pub truncated: bool,
}

impl DisconnectReason {
    pub fn from_plain(text: impl Into<String>) -> Self {
        Self::from_styled(vec![StyledTextRun {
            text: text.into(),
            style: ComponentStyle::default(),
        }])
    }

    pub fn from_styled(source: Vec<StyledTextRun>) -> Self {
        let source_runs = source.len();
        let source_text_bytes = source
            .iter()
            .fold(0usize, |total, run| total.saturating_add(run.text.len()));
        let mut remaining = MAX_DISCONNECT_REASON_RETAINED_BYTES;
        let mut retained_bytes = 0usize;
        let mut truncated = source_runs > MAX_DISCONNECT_REASON_RUNS;
        let mut runs = Vec::new();
        for run in source.into_iter().take(MAX_DISCONNECT_REASON_RUNS) {
            if remaining == 0 {
                truncated = true;
                break;
            }
            let source_run_bytes = run.text.len();
            let text = bounded_utf8(run.text, remaining);
            truncated |= text.len() < source_run_bytes;
            remaining -= text.len();
            retained_bytes += text.len();
            let font = run.style.font.and_then(|font| {
                if remaining == 0 {
                    truncated = true;
                    return None;
                }
                let source_font_bytes = font.len();
                let bounded = bounded_utf8(font, remaining.min(MAX_DISCONNECT_REASON_FONT_BYTES));
                truncated |= bounded.len() < source_font_bytes;
                remaining -= bounded.len();
                retained_bytes += bounded.len();
                Some(bounded)
            });
            runs.push(DisconnectTextRun {
                text,
                bold: run.style.bold == Some(true),
                italic: run.style.italic == Some(true),
                underlined: run.style.underlined == Some(true),
                strikethrough: run.style.strikethrough == Some(true),
                obfuscated: run.style.obfuscated == Some(true),
                color: run.style.color,
                shadow_color: run.style.shadow_color,
                font,
            });
        }
        runs.shrink_to_fit();
        Self {
            runs,
            source_runs,
            source_text_bytes,
            retained_bytes,
            truncated,
        }
    }

    pub fn summary(&self) -> String {
        let mut summary =
            String::with_capacity(self.runs.iter().map(|run| run.text.len()).sum::<usize>());
        for run in &self.runs {
            summary.push_str(&run.text);
        }
        summary
    }
}

fn bounded_utf8(mut value: String, maximum: usize) -> String {
    if value.len() > maximum {
        let mut end = maximum;
        while !value.is_char_boundary(end) {
            end -= 1;
        }
        value.truncate(end);
    }
    value.shrink_to_fit();
    value
}

#[cfg(test)]
mod disconnect_reason_tests {
    use bbb_protocol::{ComponentClickEvent, ComponentStyle, StyledTextRun};

    use super::*;

    #[test]
    fn styled_reason_is_bounded_and_drops_non_presentational_payloads() {
        let source = (0..=MAX_DISCONNECT_REASON_RUNS)
            .map(|index| StyledTextRun {
                text: if index == 0 {
                    "中".repeat(MAX_DISCONNECT_REASON_RETAINED_BYTES)
                } else {
                    "tail".to_string()
                },
                style: ComponentStyle {
                    bold: Some(true),
                    color: Some(0x12_34_56),
                    shadow_color: Some(0x8012_3456),
                    click_event: Some(ComponentClickEvent::OpenUrl {
                        url: "https://example.invalid/large".repeat(1024).into(),
                    }),
                    font: Some("minecraft:default".repeat(1024)),
                    ..ComponentStyle::default()
                },
            })
            .collect::<Vec<_>>();

        let reason = DisconnectReason::from_styled(source);

        assert!(reason.truncated);
        assert_eq!(reason.source_runs, MAX_DISCONNECT_REASON_RUNS + 1);
        assert!(reason.runs.len() <= MAX_DISCONNECT_REASON_RUNS);
        assert!(reason.retained_bytes <= MAX_DISCONNECT_REASON_RETAINED_BYTES);
        assert!(reason.summary().len() <= MAX_DISCONNECT_REASON_RETAINED_BYTES);
        assert!(reason.summary().is_char_boundary(reason.summary().len()));
        assert!(reason.runs[0].bold);
        assert_eq!(reason.runs[0].color, Some(0x12_34_56));
        assert_eq!(reason.runs[0].shadow_color, Some(0x8012_3456));
    }

    #[test]
    fn plain_reason_round_trips_without_truncation() {
        let reason = DisconnectReason::from_plain("Connection reset");
        assert_eq!(reason.summary(), "Connection reset");
        assert!(!reason.truncated);
        assert_eq!(reason.source_runs, 1);
        assert_eq!(reason.retained_bytes, 16);
        assert_eq!(reason.runs[0].shadow_color, None);
    }
}

#[derive(Debug, Default)]
pub struct InboundPacketMetrics {
    packets_seen: AtomicU64,
    inbound_bytes: AtomicU64,
}

impl InboundPacketMetrics {
    pub fn record_packet(&self, payload_len: usize) {
        self.packets_seen.fetch_add(1, Ordering::Relaxed);
        self.inbound_bytes.fetch_add(
            u64::try_from(payload_len).unwrap_or(u64::MAX),
            Ordering::Relaxed,
        );
    }

    pub fn take_delta(&self) -> (u64, u64) {
        let packets = self.packets_seen.swap(0, Ordering::AcqRel);
        let bytes = self.inbound_bytes.swap(0, Ordering::AcqRel);
        (packets, bytes)
    }

    pub fn packets_seen(&self) -> u64 {
        self.packets_seen.load(Ordering::Acquire)
    }

    pub fn inbound_bytes(&self) -> u64 {
        self.inbound_bytes.load(Ordering::Acquire)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventStreamExit {
    LocalDisconnect,
    RemoteDisconnect(DisconnectReason),
}

/// Events surfaced by the online event stream.
///
/// Clientbound play packets travel as [`NetEvent::Play`] and are applied
/// through `WorldStore::apply_play_packet`; dedicated variants exist only for
/// connection lifecycle, login/configuration-state packets, and events whose
/// response or payload requires connection or canonical world state.
#[derive(Debug)]
// Keep PlayClientbound inline so hot play events avoid per-packet heap allocation.
#[allow(clippy::large_enum_variant)]
pub enum NetEvent {
    Connected,
    Disconnected {
        reason: Option<DisconnectReason>,
    },
    StateChanged {
        state: ConnectionState,
    },
    /// RSA-4096 public keys acquired from the Minecraft Services key set.
    /// Property keys validate profile textures; certificate keys validate
    /// player profile keys before retaining remote chat sessions.
    ServicesPublicKeys {
        profile_property_keys: Vec<Vec<u8>>,
        player_certificate_keys: Vec<Vec<u8>>,
    },
    /// The server signalled play-to-configuration re-entry; inbound packet
    /// reads pause until the canonical world answers with
    /// [`crate::NetCommand::AcknowledgeConfiguration`] carrying any pending
    /// chat acknowledgement, while outbound commands continue to drain.
    StartConfiguration,
    /// The server finished sending configuration data.  The connection must
    /// remain in configuration and withhold its acknowledgement until the
    /// canonical world validates and freezes the collected registries, then
    /// answers with [`crate::NetCommand::FinishConfiguration`].
    FinishConfiguration,
    /// A position correction; the canonical world applies it and answers
    /// with [`crate::NetCommand::AcceptTeleport`] carrying the post-apply
    /// transform.  Outgoing player moves are gated until the accept is on
    /// the wire, so a slow canonical apply can never leak stale positions
    /// or stall packet reading.
    PlayerPosition {
        update: PlayerPositionUpdate,
    },
    CompressionSet {
        threshold: i32,
    },
    /// Shared atomic counters for inbound packets and bytes, delivered once
    /// per session so individual packets avoid channel/wakeup traffic.
    InboundMetrics(Arc<InboundPacketMetrics>),
    UnsupportedPacket {
        state: ConnectionState,
        packet_id: i32,
        len: usize,
    },
    CookieRequest {
        key: String,
        response_payload_present: bool,
    },
    StoreCookie {
        key: String,
        payload_len: usize,
        stored_cookie_count: usize,
    },
    ResourcePackResponse {
        id: uuid::Uuid,
        action: ResourcePackResponseAction,
    },
    SelectKnownPacks {
        known_packs: Vec<KnownPack>,
        selected_packs: Vec<KnownPack>,
    },
    CodeOfConduct {
        text: String,
    },
    // Configuration-state packets with world-owned application.
    RegistryData(RegistryData),
    ResetChat,
    UpdateEnabledFeatures(UpdateEnabledFeatures),
    UpdateTags(UpdateTags),
    CustomPayload(CustomPayload),
    CustomReportDetails(CustomReportDetails),
    ServerLinks(ServerLinks),
    Transfer(Transfer),
    ShowDialog(ShowDialog),
    ClearDialog,
    ResourcePackPush(ResourcePackPush),
    ResourcePackPrompt {
        id: uuid::Uuid,
    },
    ResourcePackPop(ResourcePackPop),
    /// A decoded clientbound play packet; applied to canonical world state via
    /// `WorldStore::apply_play_packet`.
    Play(PlayClientbound),
}
