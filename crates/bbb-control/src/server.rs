use std::{
    net::SocketAddr,
    sync::{Arc, RwLock},
};

use anyhow::{Context, Result};
use bbb_world::{BlockPos, ChunkPos};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
};

use crate::types::{
    AppStatus, CodeOfConductControlRequest, ContainerClickControlRequest,
    ContainerClickSlotControlRequest, ControlRequest, ControlResponse, ControlSnapshot,
    CreativeModeSlotControlRequest, NetControlRequest, SharedSnapshot,
};

mod params;
#[cfg(test)]
mod tests;

use self::params::{
    bool_param, change_difficulty_request_param, change_game_mode_request_param,
    edit_book_pages_param, edit_book_title_param, f32_param, i32_param, is_resource_location,
    non_empty_string_param, optional_i32_param, recipe_book_type_param, sign_lines_param,
    string_param, validate_creative_mode_slot_request, RENAME_ITEM_MAX_NAME_CHARS,
};

pub fn shared_snapshot(version: impl Into<String>) -> SharedSnapshot {
    Arc::new(RwLock::new(ControlSnapshot {
        app: AppStatus {
            version: version.into(),
            running: true,
        },
        ..ControlSnapshot::default()
    }))
}

pub async fn serve(addr: SocketAddr, snapshot: SharedSnapshot) -> Result<()> {
    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("bind control API on {addr}"))?;
    tracing::info!(%addr, "native control API listening");

    loop {
        let (stream, _) = listener.accept().await?;
        let snapshot = Arc::clone(&snapshot);
        tokio::spawn(async move {
            if let Err(err) = handle_client(stream, snapshot).await {
                tracing::debug!(?err, "control client failed");
            }
        });
    }
}

async fn handle_client(stream: TcpStream, snapshot: SharedSnapshot) -> Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();
    while let Some(line) = lines.next_line().await? {
        let response = match serde_json::from_str::<ControlRequest>(&line) {
            Ok(request) => dispatch(request, &snapshot),
            Err(err) => ControlResponse {
                ok: false,
                result: None,
                error: Some(err.to_string()),
            },
        };
        writer
            .write_all(serde_json::to_string(&response)?.as_bytes())
            .await?;
        writer.write_all(b"\n").await?;
    }
    Ok(())
}

fn dispatch(request: ControlRequest, snapshot: &SharedSnapshot) -> ControlResponse {
    if request.method == "app.quit" {
        let mut snapshot_guard = snapshot.write().expect("control snapshot poisoned");
        snapshot_guard.app.running = false;
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
        let mut snapshot_guard = snapshot.write().expect("control snapshot poisoned");
        snapshot_guard.screenshot_request = Some(path.clone());
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({"queued": true, "path": path})),
            error: None,
        };
    }

    if request.method == "net.accept_code_of_conduct" {
        let remember = bool_param(&request.params, "remember").unwrap_or(false);
        let mut snapshot_guard = snapshot.write().expect("control snapshot poisoned");
        snapshot_guard
            .code_of_conduct_requests
            .push(CodeOfConductControlRequest::Accept { remember });
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "remember": remember,
                "pending": snapshot_guard.code_of_conduct_requests.len()
            })),
            error: None,
        };
    }

    if request.method == "net.decline_code_of_conduct" {
        let mut snapshot_guard = snapshot.write().expect("control snapshot poisoned");
        snapshot_guard
            .code_of_conduct_requests
            .push(CodeOfConductControlRequest::Decline);
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": snapshot_guard.code_of_conduct_requests.len()
            })),
            error: None,
        };
    }

    if request.method == "net.clear_code_of_conduct_acceptance" {
        let mut snapshot_guard = snapshot.write().expect("control snapshot poisoned");
        snapshot_guard
            .code_of_conduct_requests
            .push(CodeOfConductControlRequest::ClearAcceptance);
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": snapshot_guard.code_of_conduct_requests.len()
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
        let mut snapshot_guard = snapshot.write().expect("control snapshot poisoned");
        snapshot_guard
            .net_requests
            .push(NetControlRequest::ChatCommand {
                command: command.to_string(),
            });
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": snapshot_guard.net_requests.len()
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
        let mut snapshot_guard = snapshot.write().expect("control snapshot poisoned");
        snapshot_guard
            .net_requests
            .push(NetControlRequest::CommandSuggestionRequest {
                id,
                command: command.to_string(),
            });
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": snapshot_guard.net_requests.len()
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
        let mut snapshot_guard = snapshot.write().expect("control snapshot poisoned");
        snapshot_guard
            .net_requests
            .push(NetControlRequest::QueryBlockEntityTag {
                transaction_id,
                x,
                y,
                z,
            });
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": snapshot_guard.net_requests.len()
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
        let mut snapshot_guard = snapshot.write().expect("control snapshot poisoned");
        snapshot_guard
            .net_requests
            .push(NetControlRequest::QueryEntityTag {
                transaction_id,
                entity_id,
            });
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": snapshot_guard.net_requests.len()
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
        let mut snapshot_guard = snapshot.write().expect("control snapshot poisoned");
        snapshot_guard
            .net_requests
            .push(NetControlRequest::SpectateEntity { entity_id });
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": snapshot_guard.net_requests.len()
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
        let mut snapshot_guard = snapshot.write().expect("control snapshot poisoned");
        snapshot_guard
            .net_requests
            .push(NetControlRequest::TeleportToEntity { uuid });
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": snapshot_guard.net_requests.len()
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
        let mut snapshot_guard = snapshot.write().expect("control snapshot poisoned");
        snapshot_guard.net_requests.push(change_difficulty);
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": snapshot_guard.net_requests.len()
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
        let mut snapshot_guard = snapshot.write().expect("control snapshot poisoned");
        snapshot_guard.net_requests.push(change_game_mode);
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": snapshot_guard.net_requests.len()
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
        let mut snapshot_guard = snapshot.write().expect("control snapshot poisoned");
        snapshot_guard
            .net_requests
            .push(NetControlRequest::LockDifficulty { locked });
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": snapshot_guard.net_requests.len()
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
        let mut snapshot_guard = snapshot.write().expect("control snapshot poisoned");
        snapshot_guard
            .net_requests
            .push(NetControlRequest::SetHeldSlot { slot });
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": snapshot_guard.net_requests.len()
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
        let mut snapshot_guard = snapshot.write().expect("control snapshot poisoned");
        snapshot_guard
            .net_requests
            .push(NetControlRequest::SetFlying { flying });
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": snapshot_guard.net_requests.len()
            })),
            error: None,
        };
    }

    if request.method == "net.perform_respawn" {
        let mut snapshot_guard = snapshot.write().expect("control snapshot poisoned");
        snapshot_guard
            .net_requests
            .push(NetControlRequest::PerformRespawn);
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": snapshot_guard.net_requests.len()
            })),
            error: None,
        };
    }

    if request.method == "net.request_stats" {
        let mut snapshot_guard = snapshot.write().expect("control snapshot poisoned");
        snapshot_guard
            .net_requests
            .push(NetControlRequest::RequestStats);
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": snapshot_guard.net_requests.len()
            })),
            error: None,
        };
    }

    if request.method == "net.request_game_rule_values" {
        let mut snapshot_guard = snapshot.write().expect("control snapshot poisoned");
        snapshot_guard
            .net_requests
            .push(NetControlRequest::RequestGameRuleValues);
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": snapshot_guard.net_requests.len()
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
        let mut snapshot_guard = snapshot.write().expect("control snapshot poisoned");
        snapshot_guard
            .net_requests
            .push(NetControlRequest::PlaceRecipe {
                container_id,
                recipe_index,
                use_max_items,
            });
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": snapshot_guard.net_requests.len()
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
        let mut snapshot_guard = snapshot.write().expect("control snapshot poisoned");
        snapshot_guard
            .net_requests
            .push(NetControlRequest::ChangeRecipeBookSettings {
                book_type,
                open,
                filtering,
            });
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": snapshot_guard.net_requests.len()
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
        let mut snapshot_guard = snapshot.write().expect("control snapshot poisoned");
        snapshot_guard
            .net_requests
            .push(NetControlRequest::MarkRecipeSeen { recipe_index });
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": snapshot_guard.net_requests.len()
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
        let mut snapshot_guard = snapshot.write().expect("control snapshot poisoned");
        snapshot_guard
            .net_requests
            .push(NetControlRequest::RenameItem {
                name: name.to_string(),
            });
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": snapshot_guard.net_requests.len()
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
        let mut snapshot_guard = snapshot.write().expect("control snapshot poisoned");
        snapshot_guard
            .net_requests
            .push(NetControlRequest::EditBook { slot, pages, title });
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": snapshot_guard.net_requests.len()
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
        let mut snapshot_guard = snapshot.write().expect("control snapshot poisoned");
        snapshot_guard
            .net_requests
            .push(NetControlRequest::SignUpdate {
                x,
                y,
                z,
                is_front_text,
                lines,
            });
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": snapshot_guard.net_requests.len()
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
        let mut snapshot_guard = snapshot.write().expect("control snapshot poisoned");
        snapshot_guard
            .net_requests
            .push(NetControlRequest::OpenAdvancementsTab {
                tab: tab.to_string(),
            });
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": snapshot_guard.net_requests.len()
            })),
            error: None,
        };
    }

    if request.method == "net.close_advancements_screen" {
        let mut snapshot_guard = snapshot.write().expect("control snapshot poisoned");
        snapshot_guard
            .net_requests
            .push(NetControlRequest::CloseAdvancementsScreen);
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": snapshot_guard.net_requests.len()
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
        let mut snapshot_guard = snapshot.write().expect("control snapshot poisoned");
        snapshot_guard
            .net_requests
            .push(NetControlRequest::SelectTrade { item });
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": snapshot_guard.net_requests.len()
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
        let mut snapshot_guard = snapshot.write().expect("control snapshot poisoned");
        snapshot_guard
            .net_requests
            .push(NetControlRequest::SetBeacon {
                primary_effect,
                secondary_effect,
            });
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": snapshot_guard.net_requests.len()
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
        let mut snapshot_guard = snapshot.write().expect("control snapshot poisoned");
        snapshot_guard
            .net_requests
            .push(NetControlRequest::SetCreativeModeSlot(request));
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": snapshot_guard.net_requests.len()
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
        let mut snapshot_guard = snapshot.write().expect("control snapshot poisoned");
        snapshot_guard
            .net_requests
            .push(NetControlRequest::SelectBundleItem {
                slot_id,
                selected_item_index,
            });
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": snapshot_guard.net_requests.len()
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
        let mut snapshot_guard = snapshot.write().expect("control snapshot poisoned");
        snapshot_guard
            .net_requests
            .push(NetControlRequest::ContainerButtonClick {
                container_id,
                button_id,
            });
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": snapshot_guard.net_requests.len()
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
        let mut snapshot_guard = snapshot.write().expect("control snapshot poisoned");
        snapshot_guard
            .net_requests
            .push(NetControlRequest::ContainerClick(click));
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": snapshot_guard.net_requests.len()
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
        let mut snapshot_guard = snapshot.write().expect("control snapshot poisoned");
        snapshot_guard
            .net_requests
            .push(NetControlRequest::ContainerClickSlot(click));
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": snapshot_guard.net_requests.len()
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
        let mut snapshot_guard = snapshot.write().expect("control snapshot poisoned");
        snapshot_guard
            .net_requests
            .push(NetControlRequest::ContainerClose { container_id });
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": snapshot_guard.net_requests.len()
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
        let mut snapshot_guard = snapshot.write().expect("control snapshot poisoned");
        snapshot_guard
            .net_requests
            .push(NetControlRequest::ContainerSlotStateChanged {
                slot_id,
                container_id,
                new_state,
            });
        return ControlResponse {
            ok: true,
            result: Some(serde_json::json!({
                "queued": true,
                "pending": snapshot_guard.net_requests.len()
            })),
            error: None,
        };
    }

    let snapshot_guard = snapshot.read().expect("control snapshot poisoned");
    let json = match request.method.as_str() {
        "app.status" => Ok(serde_json::json!({
            "app": &snapshot_guard.app,
            "net": &snapshot_guard.net,
            "audio": &snapshot_guard.audio,
            "renderer": &snapshot_guard.renderer,
            "world": snapshot_guard.world_store.counters(),
        })),
        "net.counters" => serde_json::to_value(&snapshot_guard.net),
        "audio.counters" => serde_json::to_value(&snapshot_guard.audio),
        "renderer.counters" => serde_json::to_value(&snapshot_guard.renderer),
        "world.counters" => serde_json::to_value(snapshot_guard.world_store.counters()),
        "world.apply_diagnostics" => {
            serde_json::to_value(snapshot_guard.world_store.apply_diagnostics())
        }
        "world.registries" => serde_json::to_value(snapshot_guard.world_store.registries()),
        "world.level_state" => Ok(serde_json::json!({
            "dimension": snapshot_guard.world_store.dimension(),
            "level": snapshot_guard.world_store.level_info(),
            "gameplay": snapshot_guard.world_store.gameplay(),
        })),
        "world.client_advancements" => {
            serde_json::to_value(snapshot_guard.world_store.client_advancements())
        }
        "world.client_audio" => serde_json::to_value(snapshot_guard.world_store.client_audio()),
        "world.client_chat" => serde_json::to_value(snapshot_guard.world_store.client_chat()),
        "world.client_combat" => serde_json::to_value(snapshot_guard.world_store.client_combat()),
        "world.client_cooldowns" => serde_json::to_value(snapshot_guard.world_store.cooldowns()),
        "world.client_effects" => serde_json::to_value(snapshot_guard.world_store.client_effects()),
        "world.client_command_suggestions" => {
            serde_json::to_value(snapshot_guard.world_store.client_command_suggestions())
        }
        "world.client_features" => {
            serde_json::to_value(snapshot_guard.world_store.enabled_feature_list())
        }
        "world.client_known_packs" => {
            serde_json::to_value(snapshot_guard.world_store.known_packs())
        }
        "world.client_debug_query" => {
            serde_json::to_value(snapshot_guard.world_store.client_debug_query())
        }
        "world.client_debug_game" => {
            serde_json::to_value(snapshot_guard.world_store.client_debug_game())
        }
        "world.client_hud" => serde_json::to_value(snapshot_guard.world_store.client_hud()),
        "world.client_inventory" => serde_json::to_value(snapshot_guard.world_store.inventory()),
        "world.client_local_player" => {
            serde_json::to_value(snapshot_guard.world_store.client_local_player())
        }
        "world.client_player_info" => {
            serde_json::to_value(snapshot_guard.world_store.player_info())
        }
        "world.client_recipe_book" => {
            serde_json::to_value(snapshot_guard.world_store.recipe_book())
        }
        "world.client_recipes" => serde_json::to_value(snapshot_guard.world_store.recipes()),
        "world.client_scoreboard" => serde_json::to_value(snapshot_guard.world_store.scoreboard()),
        "world.client_stats" => serde_json::to_value(snapshot_guard.world_store.client_stats()),
        "world.client_waypoints" => {
            serde_json::to_value(snapshot_guard.world_store.client_waypoints())
        }
        "world.client_ui" => serde_json::to_value(snapshot_guard.world_store.client_ui()),
        "world.client_maps" => serde_json::to_value(snapshot_guard.world_store.map_items()),
        "world.last_map_color_patch" => {
            serde_json::to_value(snapshot_guard.world_store.last_map_color_patch())
        }
        "world.command_tree" => serde_json::to_value(snapshot_guard.world_store.commands()),
        "world.last_block_changed_ack" => {
            serde_json::to_value(snapshot_guard.world_store.last_block_changed_ack())
        }
        "world.client_block_events" => Ok(serde_json::json!({
            "destructions": snapshot_guard.world_store.block_destructions(),
            "block_events": snapshot_guard.world_store.block_events(),
            "level_events": snapshot_guard.world_store.level_events(),
        })),
        "world.world_border" => serde_json::to_value(snapshot_guard.world_store.world_border()),
        "world.level_clock" => Ok(serde_json::json!({
            "world_time": snapshot_guard.world_store.world_time(),
            "weather": snapshot_guard.world_store.weather(),
            "ticking": snapshot_guard.world_store.ticking(),
        })),
        "world.chunk_view" => {
            let view = snapshot_guard.world_store.chunk_view();
            Ok(serde_json::json!({
                "first_chunk": snapshot_guard.world_store.first_chunk(),
                "center": view.center,
                "radius": view.radius,
            }))
        }
        "world.server_presentation" => {
            serde_json::to_value(snapshot_guard.world_store.presentation())
        }
        "world.probe_chunk" => {
            let x = i32_param(&request.params, "x");
            let z = i32_param(&request.params, "z");
            let result = match (x, z) {
                (Some(x), Some(z)) => snapshot_guard
                    .world_store
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
                Some(id) => snapshot_guard
                    .world_store
                    .probe_entity(id)
                    .map(|entity| serde_json::to_value(entity).expect("entity state serializes"))
                    .unwrap_or(serde_json::Value::Null),
                None => serde_json::Value::Null,
            };
            Ok(result)
        }
        "world.probe_entity_status" => {
            let id = i32_param(&request.params, "id");
            let result = match id {
                Some(id) => snapshot_guard
                    .world_store
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
                Some(id) => snapshot_guard
                    .world_store
                    .probe_entity_transform(id)
                    .map(|entity| {
                        serde_json::to_value(entity).expect("entity transform state serializes")
                    })
                    .unwrap_or(serde_json::Value::Null),
                None => serde_json::Value::Null,
            };
            Ok(result)
        }
        "world.entity_transforms" => {
            serde_json::to_value(snapshot_guard.world_store.entity_transforms())
        }
        "world.last_projectile_power" => {
            serde_json::to_value(snapshot_guard.world_store.last_projectile_power_update())
        }
        "world.entity_pick_targets" => {
            let partial_tick = f32_param(&request.params, "partial_tick").unwrap_or(1.0);
            serde_json::to_value(
                snapshot_guard
                    .world_store
                    .entity_pick_targets_at_partial_tick(partial_tick),
            )
        }
        "world.probe_block" => {
            let x = i32_param(&request.params, "x");
            let y = i32_param(&request.params, "y");
            let z = i32_param(&request.params, "z");
            let result = match (x, y, z) {
                (Some(x), Some(y), Some(z)) => snapshot_guard
                    .world_store
                    .probe_block(BlockPos { x, y, z })
                    .map(|probe| serde_json::to_value(probe).expect("block probe serializes"))
                    .unwrap_or(serde_json::Value::Null),
                _ => serde_json::Value::Null,
            };
            Ok(result)
        }
        "world.terrain_chunk_summary" => {
            let x = i32_param(&request.params, "x");
            let z = i32_param(&request.params, "z");
            let result = match (x, z) {
                (Some(x), Some(z)) => snapshot_guard
                    .world_store
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
