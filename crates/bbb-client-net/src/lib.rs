mod chat_session;
mod chunk_batch;
mod connection;
mod driver;
mod event_stream;
mod login;
mod play_bundle;
pub mod profile_textures;
mod resource_pack;
mod services_profile_keys;
pub mod session;
mod status;
mod transport;
mod types;
#[cfg(target_arch = "wasm32")]
mod wasm_fetch;

pub use crate::chunk_batch::ChunkBatchSizeCalculator;
#[cfg(not(target_arch = "wasm32"))]
pub use crate::connection::play_tick_interval;
pub use crate::connection::{RawConnection, DEFAULT_READ_TIMEOUT};
#[cfg(not(target_arch = "wasm32"))]
pub use crate::driver::{read_packet_or_eof_or_send_play_tick, read_packet_or_send_play_tick};
pub use crate::event_stream::{run_event_stream, NetEventSender};
pub use crate::login::{ensure_offline_login, send_login_hello};
pub use crate::play_bundle::{PlayPacketBatch, PlayPacketBundleAssembler};
pub use crate::profile_textures::{default_player_skin_cache_dir, ProfileNetworkService};
#[cfg(not(target_arch = "wasm32"))]
pub use crate::profile_textures::{
    player_skin_cache_path, player_texture_cache_path, write_validated_player_texture_cache,
};
pub use crate::resource_pack::{
    append_server_pack_download_log, download_server_pack, download_server_pack_with_progress,
    initialize_server_pack_download_cache, parse_expected_server_pack_sha1,
    response_action_for_push_with_policy, sha1_hex_lower, ServerPackDownload,
    ServerPackDownloadProgress, ServerPackDownloadProxy, ServerPackDownloadRequest,
    VANILLA_MAX_KEPT_SERVER_PACK_FILES, VANILLA_MAX_SERVER_PACK_DOWNLOAD_BYTES,
};
pub use crate::services_profile_keys::{
    verify_profile_texture_signature, verify_services_rsa_sha1_signature,
};
pub use crate::session::{
    ClientProtocolSession, ConfigurationPacketStep, DecodedClientboundPacket, LoginPacketStep,
    PlayPacketStep, PlayerMovementFlags,
};
pub use crate::status::status_ping;
pub use crate::transport::ClientStream;
pub use crate::types::{
    AcceptTeleportCommand, AttackEntityCommand, ClientIdentity, ConnectionOptions, ConnectionState,
    DisconnectReason, DisconnectTextRun, EventStreamExit, InboundPacketMetrics,
    InteractEntityCommand, NetCommand, NetEvent, OutboundChatCommand, OutboundChatCommandSigning,
    OutboundChatMessage, PlayerMoveCommand, ServerResourcePackPolicy, StatusPing, UseItemCommand,
    UseItemOnCommand, VehicleMoveCommand, MAX_DISCONNECT_REASON_RETAINED_BYTES,
    MAX_DISCONNECT_REASON_RUNS,
};

#[cfg(not(target_arch = "wasm32"))]
pub fn install_http_crypto_provider() {
    // reqwest 0.13's provider-free rustls feature lets BBB keep a portable,
    // explicit provider instead of inheriting the aws-lc C toolchain. Losing
    // this one-time process race is harmless: every caller installs the same
    // ring provider before constructing an HTTP client.
    let _ = rustls::crypto::ring::default_provider().install_default();
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    #[test]
    fn http_clients_use_the_explicit_ring_provider() {
        super::install_http_crypto_provider();
        let provider = rustls::crypto::CryptoProvider::get_default()
            .expect("BBB must install a rustls provider before constructing clients");
        let ring = rustls::crypto::ring::default_provider();
        assert_eq!(provider.cipher_suites, ring.cipher_suites);
        assert!(reqwest::Client::builder().build().is_ok());
    }
}
