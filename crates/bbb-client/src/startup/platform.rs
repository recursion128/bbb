use std::sync::Arc;

use anyhow::Result;
use bbb_client_assets::KnownPacksManager;
use bbb_client_net::{ClientIdentity, ConnectionOptions};
#[cfg(any(not(target_arch = "wasm32"), test))]
use bbb_client_render::{
    NAGA_VERSION, WGPU_CORE_VERSION, WGPU_HAL_VERSION, WGPU_METAL_FENCE_STRATEGY,
    WGPU_METAL_FENCE_UPSTREAM_COMMIT, WGPU_TYPES_VERSION, WGPU_VERSION,
};
#[cfg(any(not(target_arch = "wasm32"), test))]
use serde::Serialize;

use crate::{
    code_of_conduct::CodeOfConductAcceptance, network_runtime::NetworkRuntime,
    simulation::commands::LoopWake,
};

use super::{client_information_from_args, connection_options_from_args, Args};

pub(super) const fn default_fabulous_improved_transparency() -> bool {
    !cfg!(target_os = "macos")
}

fn prepare_startup_connection_options(
    args: &Args,
    identity: &ClientIdentity,
    websocket_gateway: Option<String>,
    code_of_conduct: &mut CodeOfConductAcceptance,
    known_packs_manager: Option<Arc<KnownPacksManager>>,
) -> Result<Option<ConnectionOptions>> {
    if !args.connect_server {
        return Ok(None);
    }

    let mut options = connection_options_from_args(args, identity)?;
    options.websocket_gateway = websocket_gateway;
    options.client_information = client_information_from_args(args)?;
    options.accepted_code_of_conduct_hash = code_of_conduct.accepted_hash_for_options(&options);
    options.available_known_packs = known_packs_manager
        .as_deref()
        .map(|manager| manager.available_packs().cloned().collect())
        .unwrap_or_default();
    code_of_conduct.set_connected_server(&options);
    Ok(Some(options))
}

#[cfg(any(not(target_arch = "wasm32"), test))]
#[derive(Debug, Serialize)]
pub(crate) struct BuildInfo {
    schema: u32,
    client: &'static str,
    client_version: &'static str,
    source_commit: &'static str,
    source_dirty: bool,
    minecraft_version: &'static str,
    protocol_version: i32,
    resource_pack_format: String,
    data_pack_format: String,
    target_os: &'static str,
    target_arch: &'static str,
    wgpu_version: &'static str,
    wgpu_core_version: &'static str,
    wgpu_hal_version: &'static str,
    wgpu_types_version: &'static str,
    naga_version: &'static str,
    upstream_metal_fence_commit: &'static str,
    upstream_metal_fence_strategy: &'static str,
}

#[cfg(any(not(target_arch = "wasm32"), test))]
impl BuildInfo {
    pub(super) fn current() -> Self {
        Self {
            schema: 2,
            client: "bbb-client",
            client_version: env!("CARGO_PKG_VERSION"),
            source_commit: env!("BBB_SOURCE_COMMIT"),
            source_dirty: env!("BBB_SOURCE_DIRTY") == "true",
            minecraft_version: bbb_protocol::MC_VERSION,
            protocol_version: bbb_protocol::PROTOCOL_VERSION,
            resource_pack_format: bbb_protocol::MC_RESOURCE_PACK_FORMAT.to_vanilla_string(),
            data_pack_format: bbb_protocol::MC_DATA_PACK_FORMAT.to_vanilla_string(),
            target_os: std::env::consts::OS,
            target_arch: std::env::consts::ARCH,
            wgpu_version: WGPU_VERSION,
            wgpu_core_version: WGPU_CORE_VERSION,
            wgpu_hal_version: WGPU_HAL_VERSION,
            wgpu_types_version: WGPU_TYPES_VERSION,
            naga_version: NAGA_VERSION,
            upstream_metal_fence_commit: WGPU_METAL_FENCE_UPSTREAM_COMMIT,
            upstream_metal_fence_strategy: WGPU_METAL_FENCE_STRATEGY,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::{path::PathBuf, time::Duration};

    use bbb_client_assets::PackRoots;
    use tokio::runtime::Runtime;

    use super::{
        super::{client_identity_from_args, load_known_packs_manager},
        *,
    };

    pub(crate) fn init_tracing() {
        tracing_subscriber::fmt()
            .with_writer(std::io::stderr)
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .init();
    }

    /// `Main.main` names the GUI owner `Render thread` immediately before native
    /// renderer initialization. Linux `perf` and `/proc` consume the pthread name,
    /// so use that same OS-visible identity rather than a Rust-only label.
    pub(crate) fn name_current_render_thread() -> Result<()> {
        #[cfg(target_os = "linux")]
        {
            let name = c"Render thread";
            // SAFETY: `pthread_self` identifies the current live thread, `name` is a
            // static NUL-terminated string, and its 13 bytes fit Linux's 16-byte limit.
            let result = unsafe { libc::pthread_setname_np(libc::pthread_self(), name.as_ptr()) };
            anyhow::ensure!(result == 0, "pthread_setname_np failed with errno {result}");
        }
        Ok(())
    }

    pub(crate) fn run_build_info_if_requested(args: &Args) -> Result<bool> {
        if !args.build_info {
            return Ok(false);
        }
        println!("{}", serde_json::to_string_pretty(&BuildInfo::current())?);
        Ok(true)
    }

    pub(crate) fn run_status_if_requested(runtime: &Runtime, args: &Args) -> Result<bool> {
        let Some(address) = args.status_server.as_deref() else {
            return Ok(false);
        };
        let status = runtime.block_on(bbb_client_net::status_ping(address))?;
        println!("{}", serde_json::to_string_pretty(&status)?);
        Ok(true)
    }

    pub(crate) fn run_probe_if_requested(runtime: &Runtime, args: &Args) -> Result<bool> {
        if !args.probe_server {
            return Ok(false);
        }

        let identity = client_identity_from_args(args);
        let mut connection = connection_options_from_args(args, &identity)?;
        connection.client_information = client_information_from_args(args)?;
        apply_probe_timeout(args, &mut connection);
        let roots = load_pack_roots(args)?;
        let mut options = bbb_client_probe::ProbeOptions::from_connection(connection);
        options.known_packs_manager = load_known_packs_manager(roots.as_ref());
        options.probe_after_first_chunk_packets = args.probe_after_first_chunk_packets;
        options.probe_interactions = args.probe_server_interactions;
        options.probe_operator = args.probe_server_operator;
        options.probe_dimension_transitions = args.probe_server_dimension_transitions;
        options.probe_entity_runtime = args.probe_server_entity_runtime;
        options.probe_container_menu = args.probe_server_container_menu;
        options.probe_hold = args.probe_hold_seconds.map(Duration::from_secs);
        let report = runtime.block_on(bbb_client_probe::run_probe(options))?;
        println!("{}", serde_json::to_string_pretty(&report)?);
        Ok(true)
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(in crate::startup) fn apply_probe_timeout(args: &Args, options: &mut ConnectionOptions) {
        if let Some(seconds) = args.probe_timeout_seconds {
            options.timeout = Duration::from_secs(seconds);
        }
    }

    pub(crate) fn load_pack_roots(args: &Args) -> Result<Option<PackRoots>> {
        Ok(load_pack_roots_with_options(args)?.roots)
    }

    pub(crate) struct LoadedPackRoots {
        pub(crate) roots: Option<PackRoots>,
        pub(crate) options_reconciliation:
            Option<bbb_client_assets::ClientPackOptionsReconciliation>,
    }

    pub(crate) fn load_pack_roots_with_options(args: &Args) -> Result<LoadedPackRoots> {
        load_pack_roots_with_options_inner(args, false)
    }

    /// Interactive desktop client startup requires a usable Vanilla 26.1
    /// asset root so textures, sounds, and fonts are never silently missing.
    /// Headless `--probe-server`, `--status-server`, and `--build-info` return
    /// before this runs.
    pub(crate) fn load_required_desktop_pack_roots(args: &mut Args) -> Result<LoadedPackRoots> {
        if args.vanilla_root.is_none() {
            if let Some(discovered) = crate::startup::auto_discover_vanilla_root(args) {
                tracing::info!(
                    path = %discovered.display(),
                    "auto-discovered Vanilla 26.1 root"
                );
                args.vanilla_root = Some(discovered);
            } else {
                let candidates = crate::startup::window_and_runtime::vanilla_root_candidates(args);
                return Err(missing_vanilla_assets_error(&candidates, None));
            }
        }
        load_pack_roots_with_options_inner(args, true)
    }

    fn load_pack_roots_with_options_inner(
        args: &Args,
        require_usable: bool,
    ) -> Result<LoadedPackRoots> {
        let Some(vanilla_root) = args.vanilla_root.as_ref() else {
            tracing::warn!("--vanilla-root was not supplied; Vanilla resources are unavailable");
            return Ok(LoadedPackRoots {
                roots: None,
                options_reconciliation: None,
            });
        };
        match PackRoots::from_root(vanilla_root) {
            Ok(roots) if !args.resource_pack_dirs.is_empty() => Ok(LoadedPackRoots {
                roots: Some(apply_resource_pack_dirs(
                    roots,
                    args.resource_pack_dirs.iter().cloned(),
                )?),
                options_reconciliation: None,
            }),
            Ok(roots) => {
                let reconciliation =
                    bbb_client_assets::SelectedClientPackStack::from_options_repository(
                        &roots,
                        &args.game_dir.join("resourcepacks"),
                        &args.stored_resource_packs,
                        &args.stored_incompatible_resource_packs,
                    )?;
                for id in &reconciliation.removed_missing {
                    tracing::warn!(pack_id = id, "removed missing resource pack from options");
                }
                for id in &reconciliation.removed_incompatible {
                    tracing::warn!(
                        pack_id = id,
                        "removed newly incompatible resource pack from options"
                    );
                }
                for id in &reconciliation.became_compatible {
                    tracing::info!(
                        pack_id = id,
                        "removed resource pack from incompatibility list"
                    );
                }
                let selected = reconciliation.selected_packs.clone();
                Ok(LoadedPackRoots {
                    roots: Some(roots.with_selected_client_packs(selected)?),
                    options_reconciliation: Some(reconciliation),
                })
            }
            Err(err) if require_usable => {
                let mut candidates = vec![vanilla_root.clone()];
                for candidate in crate::startup::window_and_runtime::vanilla_root_candidates(args) {
                    if !candidates.contains(&candidate) {
                        candidates.push(candidate);
                    }
                }
                Err(missing_vanilla_assets_error(&candidates, Some(err)))
            }
            Err(err) => {
                tracing::warn!(?err, "vanilla 26.1 pack roots unavailable");
                Ok(LoadedPackRoots {
                    roots: None,
                    options_reconciliation: None,
                })
            }
        }
    }

    fn missing_vanilla_assets_error(
        candidates: &[PathBuf],
        root_cause: Option<anyhow::Error>,
    ) -> anyhow::Error {
        let checked = candidates
            .iter()
            .map(|path| format!("  - {}", path.display()))
            .collect::<Vec<_>>()
            .join("\n");
        let cause_line = root_cause
            .map(|err| format!("\nDetails: {err:#}"))
            .unwrap_or_default();
        anyhow::anyhow!(
            "No usable Minecraft Java 26.1 Vanilla assets root was found.{cause_line}\n\
Checked candidate paths:\n\
{checked}\n\
To provide Vanilla 26.1 assets, do one of:\n\
  - Run `cargo dev vanilla prepare 26.1` to populate the managed cache\n\
  - Pass `--vanilla-root <PATH>` pointing to a Vanilla 26.1 assets root\n\
  - Install Minecraft Java 26.1 in your `.minecraft` directory"
        )
    }

    pub(in crate::startup) fn apply_resource_pack_dirs(
        roots: PackRoots,
        dirs: impl IntoIterator<Item = PathBuf>,
    ) -> Result<PackRoots> {
        let selected = bbb_client_assets::SelectedClientPackStack::from_startup_dirs(&roots, dirs)?;
        roots.with_selected_client_packs(selected)
    }

    pub(crate) fn start_network_if_requested(
        runtime: &Runtime,
        args: &Args,
        identity: &ClientIdentity,
        code_of_conduct: &mut CodeOfConductAcceptance,
        known_packs_manager: Option<Arc<KnownPacksManager>>,
        wake: &Arc<LoopWake>,
    ) -> Result<(NetworkRuntime, Option<ConnectionOptions>)> {
        let Some(options) = prepare_startup_connection_options(
            args,
            identity,
            None,
            code_of_conduct,
            known_packs_manager,
        )?
        else {
            return Ok((NetworkRuntime::idle(), None));
        };
        Ok((
            NetworkRuntime::start(runtime, options.clone(), Arc::clone(wake)),
            Some(options),
        ))
    }

    pub(in crate::startup) fn append_os_default_vanilla_candidates(candidates: &mut Vec<PathBuf>) {
        #[cfg(target_os = "windows")]
        {
            if let Some(local) = std::env::var_os("LOCALAPPDATA").filter(|v| !v.is_empty()) {
                candidates.push(PathBuf::from(local).join("bbb").join("vanilla"));
            }
            if let Some(appdata) = std::env::var_os("APPDATA").filter(|v| !v.is_empty()) {
                candidates.push(PathBuf::from(appdata).join(".minecraft"));
            }
        }
        #[cfg(target_os = "macos")]
        if let Some(home) = std::env::var_os("HOME").filter(|v| !v.is_empty()) {
            let home = PathBuf::from(home);
            candidates.push(
                home.join("Library")
                    .join("Caches")
                    .join("bbb")
                    .join("vanilla"),
            );
            candidates.push(
                home.join("Library")
                    .join("Application Support")
                    .join("minecraft"),
            );
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        if let Some(home) = std::env::var_os("HOME").filter(|v| !v.is_empty()) {
            let home = PathBuf::from(home);
            candidates.push(home.join(".cache").join("bbb").join("vanilla"));
            candidates.push(home.join(".minecraft"));
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(in crate::startup) use native::append_os_default_vanilla_candidates;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use native::*;
#[cfg(all(not(target_arch = "wasm32"), test))]
pub(in crate::startup) use native::{apply_probe_timeout, apply_resource_pack_dirs};

#[cfg(target_arch = "wasm32")]
pub(crate) fn start_network_web_if_requested(
    args: &Args,
    identity: &ClientIdentity,
    websocket_gateway: String,
    code_of_conduct: &mut CodeOfConductAcceptance,
    known_packs_manager: Option<Arc<KnownPacksManager>>,
    wake: &Arc<LoopWake>,
) -> Result<NetworkRuntime> {
    let Some(options) = prepare_startup_connection_options(
        args,
        identity,
        Some(websocket_gateway),
        code_of_conduct,
        known_packs_manager,
    )?
    else {
        return Ok(NetworkRuntime::idle());
    };
    Ok(NetworkRuntime::start_web(options, Arc::clone(wake)))
}
