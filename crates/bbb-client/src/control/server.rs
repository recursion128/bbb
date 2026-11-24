#[cfg(not(target_arch = "wasm32"))]
use std::{net::SocketAddr, sync::Arc, time::Duration};

#[cfg(not(target_arch = "wasm32"))]
use anyhow::{Context, Result};
use bbb_client_world::{BlockPos, ChunkPos};
#[cfg(not(target_arch = "wasm32"))]
use bbb_mcp::{
    control::MAX_CONTROL_LINE_BYTES,
    http::{read_line, trim_line, LineOutcome},
};
use bbb_protocol::to_display_json_value;
#[cfg(not(target_arch = "wasm32"))]
use tokio::{
    io::{AsyncRead, AsyncWrite, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    sync::Semaphore,
    time::timeout,
};

use crate::control::types::{
    CodeOfConductControlRequest, ContainerClickControlRequest, ContainerClickSlotControlRequest,
    ControlRequest, ControlResponse, ControlState, ControlWriteRequest,
    CreativeModeSlotControlRequest, InputControlRequest, InputKeyControl, InputMouseButtonControl,
    LookAtAnchorControl, LookAtTargetControl, NetControlRequest,
};

pub(crate) const MAX_HOLD_TICKS: u32 = 100;
pub(crate) const MAX_WAIT_TICKS: u32 = 100;
const MAX_STEP_TICKS: u64 = 100_000;
const MAX_STEP_TICK_NANOS: u64 = 10_000_000_000;
#[cfg(not(target_arch = "wasm32"))]
const MAX_CONTROL_CONNECTIONS: usize = 16;
#[cfg(not(target_arch = "wasm32"))]
const CONTROL_IO_TIMEOUT: Duration = Duration::from_secs(30);
#[cfg(not(target_arch = "wasm32"))]
const ACCEPT_ERROR_BACKOFF: Duration = Duration::from_millis(50);
#[cfg(not(target_arch = "wasm32"))]
const MAX_RETAINED_CONTROL_LINE_CAPACITY: usize = 256 * 1024;

mod params;
mod queries;
#[cfg(test)]
mod tests;

use self::params::{
    bool_param, change_difficulty_request_param, change_game_mode_request_param,
    edit_book_pages_param, edit_book_title_param, f32_param, f64_param, i32_param,
    is_resource_location, non_empty_string_param, optional_i32_param, recipe_book_type_param,
    sign_lines_param, string_param, validate_creative_mode_slot_request,
    RENAME_ITEM_MAX_NAME_CHARS,
};
use self::queries::{
    annotate_item_and_menu_names, crosshair_target_response, nearby_entities_response,
    probe_blocks_response, probe_surface_response,
};
#[cfg(test)]
pub(crate) use self::queries::{
    MAX_NEARBY_ENTITIES, MAX_WORLD_PROBE_BLOCKS, MAX_WORLD_SURFACE_COLUMNS,
};

#[cfg(not(target_arch = "wasm32"))]
pub async fn serve(addr: SocketAddr, state: ControlState) -> Result<()> {
    bbb_mcp::control::check_bind(addr).with_context(|| format!("bind control API on {addr}"))?;
    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("bind control API on {addr}"))?;
    tracing::info!(%addr, "native control API listening");
    let connections = Arc::new(Semaphore::new(MAX_CONTROL_CONNECTIONS));

    loop {
        let stream = match listener.accept().await {
            Ok((stream, _)) => stream,
            Err(err) => {
                tracing::warn!(?err, "control API accept failed; retrying");
                tokio::time::sleep(ACCEPT_ERROR_BACKOFF).await;
                continue;
            }
        };
        let Ok(permit) = Arc::clone(&connections).try_acquire_owned() else {
            drop(stream);
            continue;
        };
        let state = state.clone();
        tokio::spawn(async move {
            let _permit = permit;
            if let Err(err) = handle_client(stream, state).await {
                tracing::debug!(?err, "control client failed");
            }
        });
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn handle_client(stream: TcpStream, state: ControlState) -> Result<()> {
    handle_client_stream_with_timeout(stream, state, MAX_CONTROL_LINE_BYTES, CONTROL_IO_TIMEOUT)
        .await
}

#[cfg(not(target_arch = "wasm32"))]
async fn handle_client_stream_with_timeout<S>(
    stream: S,
    state: ControlState,
    max_line_bytes: usize,
    io_timeout: Duration,
) -> Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let _consumer = state.publication.enter_consumer();
    let (reader, mut writer) = tokio::io::split(stream);
    let mut reader = BufReader::new(reader);
    let mut line_buf = Vec::new();
    loop {
        let outcome = timeout(
            io_timeout,
            read_line(&mut reader, max_line_bytes, &mut line_buf),
        )
        .await
        .map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "control read deadline elapsed",
            )
        })??;
        match outcome {
            LineOutcome::TooLarge => {
                let message = format!("control request line exceeds {max_line_bytes} bytes");
                let response = ControlResponse {
                    ok: false,
                    result: None,
                    error: Some(message.clone()),
                };
                let encoded = serde_json::to_string(&response)?;
                let _ = timeout(io_timeout, async {
                    writer.write_all(encoded.as_bytes()).await?;
                    writer.write_all(b"\n").await?;
                    writer.flush().await
                })
                .await;
                anyhow::bail!("{message}");
            }
            LineOutcome::Eof if line_buf.is_empty() => return Ok(()),
            LineOutcome::Complete | LineOutcome::Eof => {
                let line = std::str::from_utf8(trim_line(&line_buf))
                    .context("control request line is not valid UTF-8")?;
                let response = match serde_json::from_str::<ControlRequest>(line) {
                    Ok(request) => {
                        if method_reads_snapshot(&request.method) {
                            state.ensure_fresh_snapshot().await;
                        }
                        dispatch(request, &state)
                    }
                    Err(err) => ControlResponse {
                        ok: false,
                        result: None,
                        error: Some(err.to_string()),
                    },
                };
                if line_buf.capacity() > MAX_RETAINED_CONTROL_LINE_CAPACITY {
                    line_buf = Vec::new();
                }
                let encoded = serde_json::to_string(&response)?;
                timeout(io_timeout, async {
                    writer.write_all(encoded.as_bytes()).await?;
                    writer.write_all(b"\n").await?;
                    writer.flush().await
                })
                .await
                .map_err(|_| {
                    std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        "control write deadline elapsed",
                    )
                })??;
                if outcome == LineOutcome::Eof {
                    return Ok(());
                }
            }
        }
    }
}

pub fn dispatch(request: ControlRequest, state: &ControlState) -> ControlResponse {
    dispatch_inner(request, state, &mut None)
}

pub fn dispatch_confirmed(
    request: ControlRequest,
    state: &ControlState,
) -> (
    ControlResponse,
    Option<crate::control::ControlCompletionReceiver>,
) {
    let (sender, answer) = tokio::sync::oneshot::channel();
    let mut completion = Some(sender);
    let response = dispatch_inner(request, state, &mut completion);
    (response, completion.is_none().then_some(answer))
}

/// Returns `true` when `method` reads `ControlSnapshot` rather than only
/// submitting write commands or reading lockstep atomics.
pub fn method_reads_snapshot(method: &str) -> bool {
    method.starts_with("world.")
        || matches!(
            method,
            "app.status"
                | "net.counters"
                | "audio.counters"
                | "renderer.counters"
                | "renderer.recording_status"
        )
}

fn parse_sim_step_params(params: &serde_json::Value) -> Result<(u64, u64), String> {
    let ticks = match params.get("ticks") {
        None => 1,
        Some(value) => value
            .as_u64()
            .filter(|v| (1..=MAX_STEP_TICKS).contains(v))
            .ok_or_else(|| {
                format!("sim.step requires integer param ticks in 1..={MAX_STEP_TICKS}")
            })?,
    };
    let tick_nanos = match params.get("tick_nanos") {
        None => 50_000_000,
        Some(value) => value
            .as_u64()
            .filter(|v| (1..=MAX_STEP_TICK_NANOS).contains(v))
            .ok_or_else(|| {
                format!("sim.step requires integer param tick_nanos in 1..={MAX_STEP_TICK_NANOS}")
            })?,
    };
    Ok((ticks, tick_nanos))
}

fn dispatch_inner(
    request: ControlRequest,
    state: &ControlState,
    completion: &mut Option<crate::control::ControlCompletionSender>,
) -> ControlResponse {
    macro_rules! submit_control {
        ($state:expr, $request:expr) => {{
            let request = $request;
            let submitted = match completion.take() {
                Some(sender) => $state.requests.submit_confirmed(request, sender),
                None => $state.requests.submit(request),
            };
            match submitted {
                Ok(receipt) => receipt.pending,
                Err(error) => {
                    return ControlResponse {
                        ok: false,
                        result: None,
                        error: Some(format!("control command rejected: {error}")),
                    };
                }
            }
        }};
    }

    if request.method == "sim.lockstep_status" {
        let status = bbb_client_clock::replay::status();
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "lockstep": bbb_client_clock::is_virtual(),
                "virtual_nanos": status.virtual_nanos,
                "quiescent": status.quiescent(),
                "simulation_quiescent": status.simulation_quiescent(),
                "net_frames_read": status.net_frames_read,
                "net_frames_written": status.net_frames_written,
                "net_events_emitted": status.net_events_emitted,
                "net_events_drained": status.net_events_drained,
                "commands_queued": status.commands_queued,
                "commands_applied": status.commands_applied,
                "client_ticks": status.client_ticks,
                "frames_rendered": status.frames_rendered,
                "simulation_seen_nanos": status.simulation_seen_nanos,
                "simulation_idle": status.simulation_idle,
                "state_unrendered": status.state_unrendered,
                "render_work_pending": status.render_work_pending,
            })),
            error: None,
        };
    }

    state.publication.record_demand();

    if request.method == "sim.step" {
        if !bbb_client_clock::is_virtual() {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some("sim.step requires a client started with --lockstep".to_string()),
            };
        }
        let (ticks, tick_nanos) = match parse_sim_step_params(&request.params) {
            Ok(parsed) => parsed,
            Err(error) => {
                return ControlResponse {
                    ok: false,
                    result: None,
                    error: Some(error),
                };
            }
        };
        bbb_client_clock::advance(std::time::Duration::from_nanos(
            ticks.saturating_mul(tick_nanos),
        ));
        submit_control!(state, ControlWriteRequest::LockstepWake);
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({"ticks": ticks})),
            error: None,
        };
    }

    if request.method == "sim.wait_ticks" {
        if bbb_client_clock::is_virtual() {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some(
                    "sim.wait_ticks is for real-time sessions; use sim.step in --lockstep mode"
                        .to_string(),
                ),
            };
        }
        let Some(ticks) = i32_param(&request.params, "ticks")
            .and_then(|v| u32::try_from(v).ok())
            .filter(|v| (1..=MAX_WAIT_TICKS).contains(v))
        else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some(format!(
                    "sim.wait_ticks requires integer param ticks in 1..={MAX_WAIT_TICKS}"
                )),
            };
        };
        let pending = submit_control!(state, ControlWriteRequest::WaitTicks { ticks });
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "ticks": ticks,
                "pending": pending
            })),
            error: None,
        };
    }

    if request.method == "app.quit" {
        submit_control!(state, ControlWriteRequest::Shutdown);
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({"running": false})),
            error: None,
        };
    }

    if request.method == "renderer.screenshot" {
        let Some(path) = string_param(&request.params, "path") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some("renderer.screenshot requires string param path".to_string()),
            };
        };
        let path = path.to_string();
        submit_control!(
            state,
            ControlWriteRequest::Screenshot { path: path.clone() }
        );
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({"queued": true, "path": path})),
            error: None,
        };
    }

    if request.method == "renderer.recording_start" {
        let Some(path) = non_empty_string_param(&request.params, "path") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some("renderer.recording_start requires string param path".into()),
            };
        };
        let fps = request
            .params
            .get("fps")
            .map_or(Some(30), serde_json::Value::as_u64);
        let bitrate = request
            .params
            .get("bitrate")
            .map_or(Some(8_000_000), serde_json::Value::as_u64);
        let (Some(fps @ 1..=120), Some(bitrate @ 1..=4_294_967_295)) = (fps, bitrate) else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some("recording requires 1..=120 fps and a positive u32 bitrate".into()),
            };
        };
        let pending = submit_control!(
            state,
            ControlWriteRequest::Recording(crate::control::RecordingControlRequest::Start {
                path: path.into(),
                fps: fps as u32,
                bitrate: bitrate as u32,
            })
        );
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({"queued": true, "pending": pending})),
            error: None,
        };
    }
    if request.method == "renderer.recording_stop" {
        let pending = submit_control!(
            state,
            ControlWriteRequest::Recording(crate::control::RecordingControlRequest::Stop)
        );
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({"queued": true, "pending": pending})),
            error: None,
        };
    }

    if request.method == "net.accept_code_of_conduct" {
        let remember = bool_param(&request.params, "remember").unwrap_or(false);
        let pending = submit_control!(
            state,
            ControlWriteRequest::CodeOfConduct(CodeOfConductControlRequest::Accept { remember },)
        );
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "remember": remember,
                "pending": pending
            })),
            error: None,
        };
    }

    if request.method == "net.decline_code_of_conduct" {
        let pending = submit_control!(
            state,
            ControlWriteRequest::CodeOfConduct(CodeOfConductControlRequest::Decline,)
        );
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": pending
            })),
            error: None,
        };
    }

    if request.method == "net.clear_code_of_conduct_acceptance" {
        let pending = submit_control!(
            state,
            ControlWriteRequest::CodeOfConduct(CodeOfConductControlRequest::ClearAcceptance,)
        );
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": pending
            })),
            error: None,
        };
    }

    if request.method == "net.send_chat_message" {
        let Some(message) = non_empty_string_param(&request.params, "message") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some(
                    "net.send_chat_message requires non-empty string param message".to_string(),
                ),
            };
        };
        let pending = submit_control!(
            state,
            ControlWriteRequest::Net(NetControlRequest::ChatMessage {
                message: message.to_string(),
            })
        );
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": pending
            })),
            error: None,
        };
    }

    if request.method == "net.send_chat_command" {
        let Some(command) = non_empty_string_param(&request.params, "command") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some(
                    "net.send_chat_command requires non-empty string param command".to_string(),
                ),
            };
        };
        let pending = submit_control!(
            state,
            ControlWriteRequest::Net(NetControlRequest::ChatCommand {
                command: command.to_string(),
            })
        );
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": pending
            })),
            error: None,
        };
    }

    if request.method == "net.request_command_suggestions" {
        let Some(id) = i32_param(&request.params, "id") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some(
                    "net.request_command_suggestions requires integer param id".to_string(),
                ),
            };
        };
        let Some(command) = non_empty_string_param(&request.params, "command") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some(
                    "net.request_command_suggestions requires non-empty string param command"
                        .to_string(),
                ),
            };
        };
        let pending = submit_control!(
            state,
            ControlWriteRequest::Net(NetControlRequest::CommandSuggestionRequest {
                id,
                command: command.to_string(),
            },)
        );
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": pending
            })),
            error: None,
        };
    }

    if request.method == "net.query_block_entity_tag" {
        let Some(transaction_id) = i32_param(&request.params, "transaction_id") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some(
                    "net.query_block_entity_tag requires integer param transaction_id".to_string(),
                ),
            };
        };
        let Some(x) = i32_param(&request.params, "x") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some("net.query_block_entity_tag requires integer param x".to_string()),
            };
        };
        let Some(y) = i32_param(&request.params, "y") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some("net.query_block_entity_tag requires integer param y".to_string()),
            };
        };
        let Some(z) = i32_param(&request.params, "z") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some("net.query_block_entity_tag requires integer param z".to_string()),
            };
        };
        let pending = submit_control!(
            state,
            ControlWriteRequest::Net(NetControlRequest::QueryBlockEntityTag {
                transaction_id,
                x,
                y,
                z,
            },)
        );
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": pending
            })),
            error: None,
        };
    }

    if request.method == "net.query_entity_tag" {
        let Some(transaction_id) = i32_param(&request.params, "transaction_id") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some(
                    "net.query_entity_tag requires integer param transaction_id".to_string(),
                ),
            };
        };
        let Some(entity_id) = i32_param(&request.params, "entity_id") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some("net.query_entity_tag requires integer param entity_id".to_string()),
            };
        };
        let pending = submit_control!(
            state,
            ControlWriteRequest::Net(NetControlRequest::QueryEntityTag {
                transaction_id,
                entity_id,
            },)
        );
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": pending
            })),
            error: None,
        };
    }

    if request.method == "net.spectate_entity" {
        let Some(entity_id) = i32_param(&request.params, "entity_id") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some("net.spectate_entity requires integer param entity_id".to_string()),
            };
        };
        let pending = submit_control!(
            state,
            ControlWriteRequest::Net(NetControlRequest::SpectateEntity { entity_id },)
        );
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": pending
            })),
            error: None,
        };
    }

    if request.method == "net.teleport_to_entity" {
        let Some(uuid) = string_param(&request.params, "uuid") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some("net.teleport_to_entity requires string param uuid".to_string()),
            };
        };
        let uuid = match uuid::Uuid::parse_str(uuid) {
            Ok(uuid) => uuid,
            Err(err) => {
                return ControlResponse {
                    ok: false,
                    result: None,
                    error: Some(format!(
                        "net.teleport_to_entity requires valid UUID param uuid: {err}"
                    )),
                };
            }
        };
        let pending = submit_control!(
            state,
            ControlWriteRequest::Net(NetControlRequest::TeleportToEntity { uuid },)
        );
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": pending
            })),
            error: None,
        };
    }

    if request.method == "net.change_difficulty" {
        let change_difficulty = match change_difficulty_request_param(&request.params) {
            Ok(change_difficulty) => change_difficulty,
            Err(err) => {
                return ControlResponse {
                    ok: false,
                    result: None,
                    error: Some(err),
                };
            }
        };
        let pending = submit_control!(state, ControlWriteRequest::Net(change_difficulty));
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": pending
            })),
            error: None,
        };
    }

    if request.method == "net.change_game_mode" {
        let change_game_mode = match change_game_mode_request_param(&request.params) {
            Ok(change_game_mode) => change_game_mode,
            Err(err) => {
                return ControlResponse {
                    ok: false,
                    result: None,
                    error: Some(err),
                };
            }
        };
        let pending = submit_control!(state, ControlWriteRequest::Net(change_game_mode));
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": pending
            })),
            error: None,
        };
    }

    if request.method == "net.lock_difficulty" {
        let Some(locked) = bool_param(&request.params, "locked") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some("net.lock_difficulty requires boolean param locked".to_string()),
            };
        };
        let pending = submit_control!(
            state,
            ControlWriteRequest::Net(NetControlRequest::LockDifficulty { locked },)
        );
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": pending
            })),
            error: None,
        };
    }

    if request.method == "net.set_held_slot" {
        let Some(slot) = i32_param(&request.params, "slot") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some("net.set_held_slot requires integer param slot".to_string()),
            };
        };
        if !(0..=8).contains(&slot) {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some("net.set_held_slot requires slot in range 0..=8".to_string()),
            };
        }
        let slot = slot as u8;
        let pending = submit_control!(
            state,
            ControlWriteRequest::Net(NetControlRequest::SetHeldSlot { slot })
        );
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": pending
            })),
            error: None,
        };
    }

    if request.method == "net.set_flying" {
        let Some(flying) = bool_param(&request.params, "flying") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some("net.set_flying requires boolean param flying".to_string()),
            };
        };
        let pending = submit_control!(
            state,
            ControlWriteRequest::Net(NetControlRequest::SetFlying { flying })
        );
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": pending
            })),
            error: None,
        };
    }

    if request.method == "input.focus" {
        let Some(focused) = bool_param(&request.params, "focused") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some("input.focus requires boolean param focused".to_string()),
            };
        };
        let capture_cursor = match request.params.get("capture_cursor") {
            None | Some(serde_json::Value::Null) => None,
            Some(_) => match bool_param(&request.params, "capture_cursor") {
                Some(value) => Some(value),
                None => {
                    return ControlResponse {
                        ok: false,
                        result: None,
                        error: Some(
                            "input.focus param capture_cursor must be a boolean".to_string(),
                        ),
                    };
                }
            },
        };
        let input = match capture_cursor {
            Some(capture_cursor) => InputControlRequest::FocusCapture {
                focused,
                capture_cursor,
            },
            None => InputControlRequest::Focus { focused },
        };
        let pending = submit_control!(state, ControlWriteRequest::Input(input));
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": pending
            })),
            error: None,
        };
    }

    if request.method == "input.key" {
        let Some(key) = string_param(&request.params, "key").and_then(InputKeyControl::from_name)
        else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some(format!(
                    "input.key requires param key in {}",
                    InputKeyControl::NAMES.join("/")
                )),
            };
        };
        let Some(pressed) = bool_param(&request.params, "pressed") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some("input.key requires boolean param pressed".to_string()),
            };
        };
        let hold_ticks = match request.params.get("hold_ticks") {
            None | Some(serde_json::Value::Null) => None,
            Some(_) => {
                if !pressed {
                    return ControlResponse {
                        ok: false,
                        result: None,
                        error: Some("input.key hold_ticks requires pressed=true".to_string()),
                    };
                }
                match i32_param(&request.params, "hold_ticks")
                    .and_then(|v| u32::try_from(v).ok())
                    .filter(|v| (1..=MAX_HOLD_TICKS).contains(v))
                {
                    Some(ticks) => Some(ticks),
                    None => {
                        return ControlResponse {
                            ok: false,
                            result: None,
                            error: Some(format!(
                                "input.key hold_ticks must be an integer in 1..={MAX_HOLD_TICKS}"
                            )),
                        };
                    }
                }
            }
        };
        let input = match hold_ticks {
            Some(hold_ticks) => InputControlRequest::KeyHold { key, hold_ticks },
            None => InputControlRequest::Key { key, pressed },
        };
        let pending = submit_control!(state, ControlWriteRequest::Input(input));
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": pending
            })),
            error: None,
        };
    }

    if request.method == "input.text" {
        const MAX_INPUT_TEXT_UTF16_UNITS: usize = InputControlRequest::MAX_TEXT_UTF16_UNITS;
        const MAX_INPUT_TEXT_BYTES: usize = MAX_INPUT_TEXT_UTF16_UNITS * 4;
        let Some(text) = string_param(&request.params, "text") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some("input.text requires string param text".to_string()),
            };
        };
        if text.len() > MAX_INPUT_TEXT_BYTES
            || text
                .encode_utf16()
                .take(MAX_INPUT_TEXT_UTF16_UNITS + 1)
                .count()
                > MAX_INPUT_TEXT_UTF16_UNITS
        {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some(format!(
                    "input.text text exceeds {MAX_INPUT_TEXT_UTF16_UNITS} UTF-16 units"
                )),
            };
        }
        let pending = submit_control!(
            state,
            ControlWriteRequest::Input(InputControlRequest::Text {
                text: text.to_string(),
            })
        );
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": pending
            })),
            error: None,
        };
    }

    if request.method == "input.mouse_button" {
        let Some(button) =
            string_param(&request.params, "button").and_then(InputMouseButtonControl::from_name)
        else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some(
                    "input.mouse_button requires param button in left/right/middle".to_string(),
                ),
            };
        };
        let Some(pressed) = bool_param(&request.params, "pressed") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some("input.mouse_button requires boolean param pressed".to_string()),
            };
        };
        let hold_ticks = match request.params.get("hold_ticks") {
            None | Some(serde_json::Value::Null) => None,
            Some(_) => {
                if !pressed {
                    return ControlResponse {
                        ok: false,
                        result: None,
                        error: Some(
                            "input.mouse_button hold_ticks requires pressed=true".to_string(),
                        ),
                    };
                }
                match i32_param(&request.params, "hold_ticks")
                    .and_then(|v| u32::try_from(v).ok())
                    .filter(|v| (1..=MAX_HOLD_TICKS).contains(v))
                {
                    Some(ticks) => Some(ticks),
                    None => {
                        return ControlResponse {
                            ok: false,
                            result: None,
                            error: Some(format!(
                                "input.mouse_button hold_ticks must be an integer in 1..={MAX_HOLD_TICKS}"
                            )),
                        };
                    }
                }
            }
        };
        let input = match hold_ticks {
            Some(hold_ticks) => InputControlRequest::MouseButtonHold { button, hold_ticks },
            None => InputControlRequest::MouseButton { button, pressed },
        };
        let pending = submit_control!(state, ControlWriteRequest::Input(input));
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": pending
            })),
            error: None,
        };
    }

    if request.method == "input.mouse_tap" {
        let Some(button) =
            string_param(&request.params, "button").and_then(InputMouseButtonControl::from_name)
        else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some(
                    "input.mouse_tap requires param button in left/right/middle".to_string(),
                ),
            };
        };
        let pending = submit_control!(
            state,
            ControlWriteRequest::Input(InputControlRequest::MouseTap { button })
        );
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": pending
            })),
            error: None,
        };
    }

    if request.method == "input.cursor" || request.method == "input.click" {
        let (Some(x), Some(y)) = (
            f64_param(&request.params, "x"),
            f64_param(&request.params, "y"),
        ) else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some(format!(
                    "{} requires finite window pixel params x and y",
                    request.method
                )),
            };
        };
        let input = if request.method == "input.cursor" {
            InputControlRequest::Cursor { x, y }
        } else {
            let Some(button) = string_param(&request.params, "button")
                .and_then(InputMouseButtonControl::from_name)
            else {
                return ControlResponse {
                    ok: false,
                    result: None,
                    error: Some(
                        "input.click requires param button in left/right/middle".to_string(),
                    ),
                };
            };
            InputControlRequest::Click { x, y, button }
        };
        let pending = submit_control!(state, ControlWriteRequest::Input(input));
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": pending
            })),
            error: None,
        };
    }

    if request.method == "input.scroll" {
        let Some(dy) = f32_param(&request.params, "dy") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some("input.scroll requires finite number param dy".to_string()),
            };
        };
        let dx = match request.params.get("dx") {
            None | Some(serde_json::Value::Null) => 0.0,
            Some(_) => match f32_param(&request.params, "dx") {
                Some(dx) => dx,
                None => {
                    return ControlResponse {
                        ok: false,
                        result: None,
                        error: Some("input.scroll param dx must be a finite number".to_string()),
                    };
                }
            },
        };
        let pending = submit_control!(
            state,
            ControlWriteRequest::Input(InputControlRequest::Scroll { dx, dy })
        );
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": pending
            })),
            error: None,
        };
    }

    if request.method == "input.look_delta" {
        let (Some(dx), Some(dy)) = (
            f64_param(&request.params, "dx"),
            f64_param(&request.params, "dy"),
        ) else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some("input.look_delta requires finite number params dx and dy".to_string()),
            };
        };
        let pending = submit_control!(
            state,
            ControlWriteRequest::Input(InputControlRequest::LookDelta { dx, dy })
        );
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": pending
            })),
            error: None,
        };
    }

    if request.method == "input.look" {
        let (Some(yaw), Some(pitch)) = (
            f32_param(&request.params, "yaw"),
            f32_param(&request.params, "pitch"),
        ) else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some("input.look requires finite number params yaw and pitch".to_string()),
            };
        };
        let pending = submit_control!(
            state,
            ControlWriteRequest::Input(InputControlRequest::Look { yaw, pitch })
        );
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": pending
            })),
            error: None,
        };
    }

    if request.method == "input.look_at" {
        let has_entity = request
            .params
            .get("entity_id")
            .is_some_and(|v| !v.is_null());
        let has_coords = ["x", "y", "z"]
            .iter()
            .any(|k| request.params.get(*k).is_some_and(|v| !v.is_null()));
        if has_entity && has_coords {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some(
                    "input.look_at accepts either (x, y, z) or entity_id, not both".to_string(),
                ),
            };
        }
        let target = if has_entity {
            let Some(entity_id) = i32_param(&request.params, "entity_id") else {
                return ControlResponse {
                    ok: false,
                    result: None,
                    error: Some("input.look_at requires integer param entity_id".to_string()),
                };
            };
            let anchor = match request.params.get("anchor") {
                None | Some(serde_json::Value::Null) => LookAtAnchorControl::Center,
                Some(_) => match string_param(&request.params, "anchor") {
                    Some("center") => LookAtAnchorControl::Center,
                    Some("eyes") => LookAtAnchorControl::Eyes,
                    Some("feet") => LookAtAnchorControl::Feet,
                    _ => {
                        return ControlResponse {
                            ok: false,
                            result: None,
                            error: Some(
                                "input.look_at param anchor must be center, eyes, or feet"
                                    .to_string(),
                            ),
                        };
                    }
                },
            };
            LookAtTargetControl::Entity { entity_id, anchor }
        } else {
            let (Some(x), Some(y), Some(z)) = (
                f64_param(&request.params, "x"),
                f64_param(&request.params, "y"),
                f64_param(&request.params, "z"),
            ) else {
                return ControlResponse {
                    ok: false,
                    result: None,
                    error: Some(
                        "input.look_at requires finite (x, y, z) coordinates or integer entity_id"
                            .to_string(),
                    ),
                };
            };
            LookAtTargetControl::Position { x, y, z }
        };
        let pending = submit_control!(
            state,
            ControlWriteRequest::Input(InputControlRequest::LookAt { target })
        );
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": pending
            })),
            error: None,
        };
    }

    if request.method == "net.perform_respawn" {
        let pending = submit_control!(
            state,
            ControlWriteRequest::Net(NetControlRequest::PerformRespawn)
        );
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": pending
            })),
            error: None,
        };
    }

    if request.method == "net.request_stats" {
        let pending = submit_control!(
            state,
            ControlWriteRequest::Net(NetControlRequest::RequestStats)
        );
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": pending
            })),
            error: None,
        };
    }

    if request.method == "net.request_game_rule_values" {
        let pending = submit_control!(
            state,
            ControlWriteRequest::Net(NetControlRequest::RequestGameRuleValues,)
        );
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": pending
            })),
            error: None,
        };
    }

    if request.method == "net.place_recipe" {
        let Some(container_id) = i32_param(&request.params, "container_id") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some("net.place_recipe requires integer param container_id".to_string()),
            };
        };
        let Some(recipe_index) = i32_param(&request.params, "recipe_index") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some("net.place_recipe requires integer param recipe_index".to_string()),
            };
        };
        let Some(use_max_items) = bool_param(&request.params, "use_max_items") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some("net.place_recipe requires boolean param use_max_items".to_string()),
            };
        };
        let pending = submit_control!(
            state,
            ControlWriteRequest::Net(NetControlRequest::PlaceRecipe {
                container_id,
                recipe_index,
                use_max_items,
            })
        );
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": pending
            })),
            error: None,
        };
    }

    if request.method == "net.change_recipe_book_settings" {
        let Some(book_type) = recipe_book_type_param(&request.params, "book_type") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some(
                    "net.change_recipe_book_settings requires book_type crafting, furnace, blast_furnace, or smoker"
                        .to_string(),
                ),
            };
        };
        let Some(open) = bool_param(&request.params, "open") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some(
                    "net.change_recipe_book_settings requires boolean param open".to_string(),
                ),
            };
        };
        let Some(filtering) = bool_param(&request.params, "filtering") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some(
                    "net.change_recipe_book_settings requires boolean param filtering".to_string(),
                ),
            };
        };
        let pending = submit_control!(
            state,
            ControlWriteRequest::Net(NetControlRequest::ChangeRecipeBookSettings {
                book_type,
                open,
                filtering,
            },)
        );
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": pending
            })),
            error: None,
        };
    }

    if request.method == "net.mark_recipe_seen" {
        let Some(recipe_index) = i32_param(&request.params, "recipe_index") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some("net.mark_recipe_seen requires integer param recipe_index".to_string()),
            };
        };
        let pending = submit_control!(
            state,
            ControlWriteRequest::Net(NetControlRequest::MarkRecipeSeen { recipe_index },)
        );
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": pending
            })),
            error: None,
        };
    }

    if request.method == "net.rename_item" {
        let Some(name) = string_param(&request.params, "name") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some("net.rename_item requires string param name".to_string()),
            };
        };
        if name.chars().count() > RENAME_ITEM_MAX_NAME_CHARS {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some(format!(
                    "net.rename_item name exceeds {RENAME_ITEM_MAX_NAME_CHARS} characters"
                )),
            };
        }
        let pending = submit_control!(
            state,
            ControlWriteRequest::Net(NetControlRequest::RenameItem {
                name: name.to_string(),
            })
        );
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": pending
            })),
            error: None,
        };
    }

    if request.method == "net.edit_book" {
        let Some(slot) = i32_param(&request.params, "slot") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some("net.edit_book requires integer param slot".to_string()),
            };
        };
        let pages = match edit_book_pages_param(&request.params, "pages") {
            Ok(pages) => pages,
            Err(err) => {
                return ControlResponse {
                    ok: false,
                    result: None,
                    error: Some(err),
                };
            }
        };
        let title = match edit_book_title_param(&request.params, "title") {
            Ok(title) => title,
            Err(err) => {
                return ControlResponse {
                    ok: false,
                    result: None,
                    error: Some(err),
                };
            }
        };
        let pending = submit_control!(
            state,
            ControlWriteRequest::Net(NetControlRequest::EditBook { slot, pages, title })
        );
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": pending
            })),
            error: None,
        };
    }

    if request.method == "net.update_sign" {
        let Some(x) = i32_param(&request.params, "x") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some("net.update_sign requires integer param x".to_string()),
            };
        };
        let Some(y) = i32_param(&request.params, "y") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some("net.update_sign requires integer param y".to_string()),
            };
        };
        let Some(z) = i32_param(&request.params, "z") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some("net.update_sign requires integer param z".to_string()),
            };
        };
        let Some(is_front_text) = bool_param(&request.params, "is_front_text") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some("net.update_sign requires boolean param is_front_text".to_string()),
            };
        };
        let lines = match sign_lines_param(&request.params, "lines") {
            Ok(lines) => lines,
            Err(err) => {
                return ControlResponse {
                    ok: false,
                    result: None,
                    error: Some(err),
                };
            }
        };
        let pending = submit_control!(
            state,
            ControlWriteRequest::Net(NetControlRequest::SignUpdate {
                x,
                y,
                z,
                is_front_text,
                lines,
            })
        );
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": pending
            })),
            error: None,
        };
    }

    if request.method == "net.open_advancements_tab" {
        let Some(tab) = string_param(&request.params, "tab") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some("net.open_advancements_tab requires string param tab".to_string()),
            };
        };
        if !is_resource_location(tab) {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some(
                    "net.open_advancements_tab requires vanilla-style resource location param tab"
                        .to_string(),
                ),
            };
        }
        let pending = submit_control!(
            state,
            ControlWriteRequest::Net(NetControlRequest::OpenAdvancementsTab {
                tab: tab.to_string(),
            },)
        );
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": pending
            })),
            error: None,
        };
    }

    if request.method == "net.close_advancements_screen" {
        let pending = submit_control!(
            state,
            ControlWriteRequest::Net(NetControlRequest::CloseAdvancementsScreen,)
        );
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": pending
            })),
            error: None,
        };
    }

    if request.method == "net.select_trade" {
        let Some(item) = i32_param(&request.params, "item") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some("net.select_trade requires integer param item".to_string()),
            };
        };
        if item < 0 {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some("net.select_trade requires item >= 0".to_string()),
            };
        }
        let pending = submit_control!(
            state,
            ControlWriteRequest::Net(NetControlRequest::SelectTrade { item })
        );
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": pending
            })),
            error: None,
        };
    }

    if request.method == "net.set_beacon" {
        let primary_effect = match optional_i32_param(&request.params, "primary_effect") {
            Ok(effect) => effect,
            Err(err) => {
                return ControlResponse {
                    ok: false,
                    result: None,
                    error: Some(err),
                };
            }
        };
        let secondary_effect = match optional_i32_param(&request.params, "secondary_effect") {
            Ok(effect) => effect,
            Err(err) => {
                return ControlResponse {
                    ok: false,
                    result: None,
                    error: Some(err),
                };
            }
        };
        let pending = submit_control!(
            state,
            ControlWriteRequest::Net(NetControlRequest::SetBeacon {
                primary_effect,
                secondary_effect,
            })
        );
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": pending
            })),
            error: None,
        };
    }

    if request.method == "net.set_creative_mode_slot" {
        let request = match serde_json::from_value::<CreativeModeSlotControlRequest>(
            request.params.clone(),
        ) {
            Ok(request) => request,
            Err(err) => {
                return ControlResponse {
                    ok: false,
                    result: None,
                    error: Some(format!("net.set_creative_mode_slot invalid params: {err}")),
                };
            }
        };
        if let Err(err) = validate_creative_mode_slot_request(&request) {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some(err),
            };
        }
        let pending = submit_control!(
            state,
            ControlWriteRequest::Net(NetControlRequest::SetCreativeModeSlot(request),)
        );
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": pending
            })),
            error: None,
        };
    }

    if request.method == "net.select_bundle_item" {
        let Some(slot_id) = i32_param(&request.params, "slot_id") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some("net.select_bundle_item requires integer param slot_id".to_string()),
            };
        };
        let Some(selected_item_index) = i32_param(&request.params, "selected_item_index") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some(
                    "net.select_bundle_item requires integer param selected_item_index".to_string(),
                ),
            };
        };
        if selected_item_index < -1 {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some(
                    "net.select_bundle_item requires selected_item_index >= 0 or -1".to_string(),
                ),
            };
        }
        let pending = submit_control!(
            state,
            ControlWriteRequest::Net(NetControlRequest::SelectBundleItem {
                slot_id,
                selected_item_index,
            },)
        );
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": pending
            })),
            error: None,
        };
    }

    if request.method == "net.container_button_click" {
        let Some(container_id) = i32_param(&request.params, "container_id") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some(
                    "net.container_button_click requires integer param container_id".to_string(),
                ),
            };
        };
        let Some(button_id) = i32_param(&request.params, "button_id") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some(
                    "net.container_button_click requires integer param button_id".to_string(),
                ),
            };
        };
        let pending = submit_control!(
            state,
            ControlWriteRequest::Net(NetControlRequest::ContainerButtonClick {
                container_id,
                button_id,
            },)
        );
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": pending
            })),
            error: None,
        };
    }

    if request.method == "net.container_click" {
        let click =
            match serde_json::from_value::<ContainerClickControlRequest>(request.params.clone()) {
                Ok(click) => click,
                Err(err) => {
                    return ControlResponse {
                        ok: false,
                        result: None,
                        error: Some(format!("net.container_click invalid params: {err}")),
                    };
                }
            };
        let pending = submit_control!(
            state,
            ControlWriteRequest::Net(NetControlRequest::ContainerClick(click,))
        );
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": pending
            })),
            error: None,
        };
    }

    if request.method == "net.container_click_slot" {
        let click = match serde_json::from_value::<ContainerClickSlotControlRequest>(
            request.params.clone(),
        ) {
            Ok(click) => click,
            Err(err) => {
                return ControlResponse {
                    ok: false,
                    result: None,
                    error: Some(format!("net.container_click_slot invalid params: {err}")),
                };
            }
        };
        let pending = submit_control!(
            state,
            ControlWriteRequest::Net(NetControlRequest::ContainerClickSlot(click),)
        );
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": pending
            })),
            error: None,
        };
    }

    if request.method == "net.container_close" {
        let Some(container_id) = i32_param(&request.params, "container_id") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some("net.container_close requires integer param container_id".to_string()),
            };
        };
        let pending = submit_control!(
            state,
            ControlWriteRequest::Net(NetControlRequest::ContainerClose { container_id },)
        );
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": pending
            })),
            error: None,
        };
    }

    if request.method == "net.container_slot_state_changed" {
        let Some(slot_id) = i32_param(&request.params, "slot_id") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some(
                    "net.container_slot_state_changed requires integer param slot_id".to_string(),
                ),
            };
        };
        let Some(container_id) = i32_param(&request.params, "container_id") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some(
                    "net.container_slot_state_changed requires integer param container_id"
                        .to_string(),
                ),
            };
        };
        let Some(new_state) = bool_param(&request.params, "new_state") else {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some(
                    "net.container_slot_state_changed requires boolean param new_state".to_string(),
                ),
            };
        };
        let pending = submit_control!(
            state,
            ControlWriteRequest::Net(NetControlRequest::ContainerSlotStateChanged {
                slot_id,
                container_id,
                new_state,
            },)
        );
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": pending
            })),
            error: None,
        };
    }

    let snapshot_view = state.snapshot.load();
    let world_snapshot = &snapshot_view.world_view;
    let snapshot_json: Option<serde_json::Result<serde_json::Value>> = match request.method.as_str()
    {
        "app.status" => Some(Ok(serde_json::json!({
            "app": &snapshot_view.app,
            "net": &snapshot_view.net,
            "audio": &snapshot_view.audio,
            "renderer": &snapshot_view.renderer,
            "world": world_snapshot.world.counters(),
        }))),
        "net.counters" => Some(serde_json::to_value(&snapshot_view.net)),
        "audio.counters" => Some(serde_json::to_value(&snapshot_view.audio)),
        "renderer.counters" => Some(serde_json::to_value(&snapshot_view.renderer)),
        "renderer.recording_status" => {
            Some(serde_json::to_value(&snapshot_view.renderer.recording))
        }
        "world.counters" => Some(serde_json::to_value(world_snapshot.world.counters())),
        "world.client_local_player" => Some(
            to_display_json_value(world_snapshot.world.client_local_player()).map(|mut v| {
                annotate_item_and_menu_names(&mut v);
                v
            }),
        ),
        "world.probe_block" => {
            let x = i32_param(&request.params, "x");
            let y = i32_param(&request.params, "y");
            let z = i32_param(&request.params, "z");
            let result = match (x, y, z) {
                (Some(x), Some(y), Some(z)) => world_snapshot
                    .world
                    .probe_block(BlockPos { x, y, z })
                    .map(|probe| serde_json::to_value(probe).expect("block probe serializes"))
                    .unwrap_or(serde_json::Value::Null),
                _ => serde_json::Value::Null,
            };
            Some(Ok(result))
        }
        _ => None,
    };
    if let Some(json) = snapshot_json {
        return control_response_from_json(json);
    }

    let world_guard = world_snapshot.world.as_ref();
    match request.method.as_str() {
        "world.probe_blocks" => return probe_blocks_response(world_guard, &request.params),
        "world.probe_surface" => return probe_surface_response(world_guard, &request.params),
        "world.crosshair_target" => return crosshair_target_response(world_guard),
        "world.nearby_entities" => return nearby_entities_response(world_guard, &request.params),
        _ => {}
    }
    let json = match request.method.as_str() {
        "world.apply_diagnostics" => serde_json::to_value(world_guard.apply_diagnostics()),
        "world.registries" => serde_json::to_value(world_guard.registries()),
        "world.level_state" => Ok(serde_json::json!({
            "dimension": world_guard.dimension(),
            "level": world_guard.level_info(),
            "gameplay": world_guard.gameplay(),
        })),
        "world.client_advancements" => to_display_json_value(world_guard.client_advancements()),
        "world.client_audio" => serde_json::to_value(world_guard.client_audio()),
        "world.client_chat" => to_display_json_value(world_guard.client_chat()),
        "world.client_combat" => to_display_json_value(world_guard.client_combat()),
        "world.client_cooldowns" => serde_json::to_value(world_guard.cooldowns()),
        "world.client_effects" => serde_json::to_value(world_guard.client_effects()),
        "world.client_command_suggestions" => {
            to_display_json_value(world_guard.client_command_suggestions())
        }
        "world.client_features" => serde_json::to_value(world_guard.enabled_feature_list()),
        "world.client_known_packs" => serde_json::to_value(world_guard.known_packs()),
        "world.client_debug_query" => serde_json::to_value(world_guard.client_debug_query()),
        "world.client_debug_game" => to_display_json_value(world_guard.client_debug_game()),
        "world.client_hud" => to_display_json_value(world_guard.client_hud()),
        "world.client_inventory" => to_display_json_value(world_guard.inventory()).map(|mut v| {
            annotate_item_and_menu_names(&mut v);
            v
        }),
        "world.client_player_info" => to_display_json_value(world_guard.player_info()),
        "world.client_recipe_book" => {
            to_display_json_value(world_guard.recipe_book()).map(|mut v| {
                annotate_item_and_menu_names(&mut v);
                v
            })
        }
        "world.client_recipes" => to_display_json_value(world_guard.recipes()),
        "world.client_scoreboard" => to_display_json_value(world_guard.scoreboard()),
        "world.client_stats" => serde_json::to_value(world_guard.client_stats()),
        "world.client_waypoints" => serde_json::to_value(world_guard.client_waypoints()),
        "world.client_ui" => Ok(world_snapshot.client_ui.clone()).map(|mut ui| {
            if let Some(creative_inventory) = &world_snapshot.creative_inventory {
                ui["current_creative_inventory"] = creative_inventory.clone();
            }
            ui
        }),
        "world.client_maps" => to_display_json_value(world_guard.map_items()),
        "world.last_map_color_patch" => serde_json::to_value(world_guard.last_map_color_patch()),
        "world.command_tree" => serde_json::to_value(world_guard.commands()),
        "world.last_block_changed_ack" => {
            serde_json::to_value(world_guard.last_block_changed_ack())
        }
        "world.client_block_events" => Ok(serde_json::json!({
            "destructions": world_guard.block_destructions(),
            "block_events": world_guard.block_events(),
            "level_events": world_guard.level_events(),
        })),
        "world.world_border" => serde_json::to_value(world_guard.world_border()),
        "world.level_clock" => Ok(serde_json::json!({
            "world_time": world_guard.world_time(),
            "weather": world_guard.weather(),
            "ticking": world_guard.ticking(),
        })),
        "world.chunk_view" => {
            let view = world_guard.chunk_view();
            Ok(serde_json::json!({
                "first_chunk": world_guard.first_chunk(),
                "center": view.center,
                "radius": view.radius,
            }))
        }
        "world.server_presentation" => to_display_json_value(world_guard.presentation()),
        "world.probe_chunk" => {
            let x = i32_param(&request.params, "x");
            let z = i32_param(&request.params, "z");
            let result = match (x, z) {
                (Some(x), Some(z)) => world_guard
                    .probe_chunk_summary(ChunkPos { x, z })
                    .map(|summary| {
                        serde_json::to_value(summary).expect("chunk probe summary serializes")
                    })
                    .unwrap_or(serde_json::Value::Null),
                _ => serde_json::Value::Null,
            };
            Ok(result)
        }
        "world.probe_entity" => {
            let id = i32_param(&request.params, "id");
            let result = match id {
                Some(id) => world_guard
                    .probe_entity(id)
                    .map(|entity| to_display_json_value(&entity).expect("entity state serializes"))
                    .unwrap_or(serde_json::Value::Null),
                None => serde_json::Value::Null,
            };
            Ok(result)
        }
        "world.probe_entity_status" => {
            let id = i32_param(&request.params, "id");
            let result = match id {
                Some(id) => world_guard
                    .probe_entity_status(id)
                    .map(|status| {
                        serde_json::to_value(status).expect("entity status probe serializes")
                    })
                    .unwrap_or(serde_json::Value::Null),
                None => serde_json::Value::Null,
            };
            Ok(result)
        }
        "world.probe_entity_transform" => {
            let id = i32_param(&request.params, "id");
            let result = match id {
                Some(id) => world_guard
                    .probe_entity_transform(id)
                    .map(|entity| {
                        serde_json::to_value(entity).expect("entity transform state serializes")
                    })
                    .unwrap_or(serde_json::Value::Null),
                None => serde_json::Value::Null,
            };
            Ok(result)
        }
        "world.entity_transforms" => serde_json::to_value(world_guard.entity_transforms()),
        "world.last_projectile_power" => {
            serde_json::to_value(world_guard.last_projectile_power_update())
        }
        "world.entity_pick_targets" => {
            let partial_tick = f32_param(&request.params, "partial_tick").unwrap_or(1.0);
            serde_json::to_value(world_guard.entity_pick_targets_at_partial_tick(partial_tick))
        }
        "world.terrain_chunk_summary" => {
            let x = i32_param(&request.params, "x");
            let z = i32_param(&request.params, "z");
            let result = match (x, z) {
                (Some(x), Some(z)) => world_guard
                    .extract_terrain_chunk(ChunkPos { x, z })
                    .map(|snapshot| {
                        serde_json::to_value(snapshot.summary())
                            .expect("terrain summary serializes")
                    })
                    .unwrap_or(serde_json::Value::Null),
                _ => serde_json::Value::Null,
            };
            Ok(result)
        }
        _ => {
            return ControlResponse {
                ok: false,
                result: None,
                error: Some(format!("unknown method {}", request.method)),
            };
        }
    };

    control_response_from_json(json)
}

fn control_response_from_json(json: serde_json::Result<serde_json::Value>) -> ControlResponse {
    match json {
        Ok(value) => ControlResponse {
            ok: true,
            result: Some(value),
            error: None,
        },
        Err(err) => ControlResponse {
            ok: false,
            result: None,
            error: Some(err.to_string()),
        },
    }
}
