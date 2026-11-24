use super::*;
use bbb_protocol::packets::{ResourcePackPop, ResourcePackPush};
use std::{
    fs::File,
    io::Write,
    time::{SystemTime, UNIX_EPOCH},
};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

fn test_runtime(label: &str) -> ServerResourcePackRuntime {
    ServerResourcePackRuntime::with_optional_runtime(
        None,
        unique_temp_dir(label),
        "bbb-client".to_string(),
        Uuid::nil(),
        None,
    )
}

#[test]
fn default_server_pack_cache_uses_vanilla_game_directory_downloads() {
    assert_eq!(
        default_server_resource_pack_cache_dir(Path::new("/game")),
        PathBuf::from("/game/downloads")
    );
}

#[test]
fn server_pack_runtime_initializes_the_exact_vanilla_download_queue_root() {
    let async_runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let cache_dir = unique_temp_dir("queue-root");
    let runtime = ServerResourcePackRuntime::new(
        async_runtime.handle().clone(),
        &cache_dir,
        "bbb-client",
        Uuid::nil(),
        None,
    )
    .unwrap();

    assert_eq!(runtime.download_cache_dir, cache_dir);
    assert!(runtime.download_cache_dir.join("log.json").is_file());
    assert!(!runtime.download_cache_dir.join("downloads").exists());

    drop(runtime);
    std::fs::remove_dir_all(cache_dir).unwrap();
}

#[test]
fn failed_runtime_download_writes_vanilla_download_queue_event() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    let url = format!("http://{address}/failed.zip");
    let pack_id = Uuid::from_u128(9);
    let mut world = WorldStore::new();
    push_accepted(&mut world, pack_id, &url);
    let async_runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let cache_dir = unique_temp_dir("queue-event");
    let mut runtime = ServerResourcePackRuntime::new(
        async_runtime.handle().clone(),
        &cache_dir,
        "bbb-client",
        Uuid::nil(),
        None,
    )
    .unwrap();

    async_runtime.block_on(async {
        runtime.synchronize(&mut world, &None, true);
        for _ in 0..200 {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            runtime.synchronize(&mut world, &None, true);
            if std::fs::read_to_string(cache_dir.join("log.json"))
                .unwrap()
                .lines()
                .next()
                .is_some()
            {
                return;
            }
        }
        panic!("server pack failure did not reach the download queue event log");
    });

    let raw = std::fs::read_to_string(cache_dir.join("log.json")).unwrap();
    let entries = raw.lines().collect::<Vec<_>>();
    assert_eq!(entries.len(), 1);
    let event: serde_json::Value = serde_json::from_str(entries[0]).unwrap();
    assert_eq!(event["id"], pack_id.to_string());
    assert_eq!(event["url"], url);
    assert_eq!(event["error"], "download_failed");

    drop(runtime);
    std::fs::remove_dir_all(cache_dir).unwrap();
}

fn push_accepted(world: &mut WorldStore, id: Uuid, url: &str) -> PackKey {
    world.apply_resource_pack_push(ResourcePackPush {
        id,
        url: url.to_string(),
        hash: String::new(),
        required: false,
        prompt: None,
    });
    world.apply_resource_pack_response(id, ResourcePackResponseAction::Accepted);
    PackKey::from_world(world.resource_pack(id).unwrap())
}

#[test]
fn accepted_packs_are_tracked_in_push_order() {
    let mut world = WorldStore::new();
    let first = push_accepted(
        &mut world,
        Uuid::from_u128(1),
        "https://example.test/first.zip",
    );
    let second = push_accepted(
        &mut world,
        Uuid::from_u128(2),
        "https://example.test/second.zip",
    );
    let mut runtime = test_runtime("order");

    runtime.reconcile_world(&world, true);

    assert_eq!(
        runtime
            .packs
            .iter()
            .map(|pack| pack.key)
            .collect::<Vec<_>>(),
        vec![first, second]
    );
    // Packs accepted together in one pass share one download batch per Vanilla
    // `ServerPackManager.updateDownloads`.
    assert_eq!(runtime.download_batches.len(), 1);
    assert_eq!(runtime.download_batches[0].members, vec![first, second]);
    assert_eq!(
        runtime.download_batches[0]
            .requests
            .iter()
            .map(|request| request.key)
            .collect::<Vec<_>>(),
        vec![first, second]
    );
    assert!(!runtime.download_batches[0].started);
}

#[test]
fn download_progress_preserves_batch_counts_and_terminal_failure() {
    let mut world = WorldStore::new();
    let first = push_accepted(
        &mut world,
        Uuid::from_u128(1),
        "https://example.test/first.zip",
    );
    let second = push_accepted(
        &mut world,
        Uuid::from_u128(2),
        "https://example.test/second.zip",
    );
    let mut runtime = test_runtime("progress");
    runtime.reconcile_world(&world, true);
    assert_eq!(runtime.download_batches.len(), 1);
    assert_eq!(runtime.download_batches[0].members, vec![first, second]);
    let batch_id = runtime.download_batches[0].id;

    for progress in [
        ServerPackDownloadProgress::RequestStarted,
        ServerPackDownloadProgress::DownloadStarted {
            total_bytes: Some(1_024),
        },
        ServerPackDownloadProgress::DownloadedBytes { bytes: 512 },
        ServerPackDownloadProgress::RequestFinished { success: false },
    ] {
        runtime.apply_task_event(PackTaskEvent::DownloadProgress {
            batch_id,
            key: first,
            current_pack: 1,
            total_packs: 2,
            progress,
        });
    }
    runtime.apply_task_event(PackTaskEvent::DownloadProgress {
        batch_id,
        key: second,
        current_pack: 2,
        total_packs: 2,
        progress: ServerPackDownloadProgress::RequestFinished { success: true },
    });
    runtime.publish_runtime_diagnostics(&mut world);

    let progress = world.resource_pack_download_state().unwrap();
    assert_eq!(progress.batch_sequence, batch_id);
    assert_eq!(progress.current_pack, 2);
    assert_eq!(progress.total_packs, 2);
    assert_eq!(progress.failed_packs, 1);
    assert_eq!(progress.phase, ResourcePackDownloadPhase::Failed);
    assert_eq!(
        progress.toast_action,
        bbb_client_world::ResourcePackDownloadToastAction::AddOrUpdateTitle
    );
}

#[test]
fn download_progress_marks_only_vanilla_toast_mutation_callbacks() {
    let mut world = WorldStore::new();
    let key = push_accepted(
        &mut world,
        Uuid::from_u128(1),
        "https://example.test/first.zip",
    );
    let mut runtime = test_runtime("progress-toast-actions");
    runtime.reconcile_world(&world, true);
    let batch_id = runtime.download_batches[0].id;

    runtime.apply_download_progress(
        batch_id,
        key,
        1,
        2,
        ServerPackDownloadProgress::RequestStarted,
    );
    assert_eq!(
        runtime.download_presentations[0].toast_action,
        bbb_client_world::ResourcePackDownloadToastAction::AddOrUpdateTitlePreserveMessage
    );
    runtime.apply_download_progress(
        batch_id,
        key,
        1,
        2,
        ServerPackDownloadProgress::DownloadStarted {
            total_bytes: Some(1_024),
        },
    );
    assert_eq!(
        runtime.download_presentations[0].toast_action,
        bbb_client_world::ResourcePackDownloadToastAction::AddOrUpdateProgress
    );
    runtime.apply_download_progress(
        batch_id,
        key,
        1,
        2,
        ServerPackDownloadProgress::RequestFinished { success: true },
    );
    assert_eq!(
        runtime.download_presentations[0].toast_action,
        bbb_client_world::ResourcePackDownloadToastAction::None,
        "non-terminal requestFinished does not call updateToast"
    );
    runtime.apply_download_progress(
        batch_id,
        key,
        2,
        2,
        ServerPackDownloadProgress::RequestStarted,
    );
    let second_request = &runtime.download_presentations[0];
    assert_eq!(second_request.downloaded_bytes, 0);
    assert_eq!(second_request.total_bytes, Some(1_024));
    assert_eq!(
        second_request.toast_action,
        bbb_client_world::ResourcePackDownloadToastAction::AddOrUpdateTitlePreserveMessage
    );
}

#[test]
fn download_progress_ignores_a_pack_from_another_sequential_batch() {
    let mut world = WorldStore::new();
    let mut runtime = test_runtime("progress-partition");
    let first = push_accepted(
        &mut world,
        Uuid::from_u128(1),
        "https://example.test/first.zip",
    );
    runtime.reconcile_world(&world, true);
    let second = push_accepted(
        &mut world,
        Uuid::from_u128(2),
        "https://example.test/second.zip",
    );
    runtime.reconcile_world(&world, true);
    let first_batch = runtime.download_batches[0].id;
    let second_batch = runtime.download_batches[1].id;

    runtime.apply_task_event(PackTaskEvent::DownloadProgress {
        batch_id: first_batch,
        key: first,
        current_pack: 1,
        total_packs: 1,
        progress: ServerPackDownloadProgress::RequestStarted,
    });
    runtime.apply_task_event(PackTaskEvent::DownloadProgress {
        batch_id: first_batch,
        key: first,
        current_pack: 1,
        total_packs: 1,
        progress: ServerPackDownloadProgress::RequestFinished { success: false },
    });
    runtime.apply_task_event(PackTaskEvent::DownloadProgress {
        batch_id: first_batch,
        key: second,
        current_pack: 1,
        total_packs: 1,
        progress: ServerPackDownloadProgress::RequestStarted,
    });
    assert_eq!(runtime.download_presentations.len(), 1);
    assert_eq!(
        runtime.download_presentations[0].batch_sequence,
        first_batch
    );
    assert_eq!(
        runtime.download_presentations[0].phase,
        ResourcePackDownloadPhase::Failed
    );
    runtime.apply_task_event(PackTaskEvent::DownloadProgress {
        batch_id: second_batch,
        key: second,
        current_pack: 1,
        total_packs: 1,
        progress: ServerPackDownloadProgress::RequestStarted,
    });
    assert_eq!(
        runtime
            .download_presentations
            .last()
            .unwrap()
            .batch_sequence,
        second_batch
    );
    runtime.publish_runtime_diagnostics(&mut world);
    assert_eq!(world.resource_pack_download_states().len(), 2);
    assert_eq!(
        world.resource_pack_download_states()[0].batch_sequence,
        first_batch
    );
    assert_eq!(
        world.resource_pack_download_states()[1].batch_sequence,
        second_batch
    );
}

#[test]
fn dropping_runtime_removes_materialized_session_directory() {
    let runtime = test_runtime("drop-cleanup");
    let session_dir = runtime.materialized_session_dir.clone();
    let cache_root = runtime.download_cache_dir.clone();
    std::fs::create_dir_all(&session_dir).unwrap();
    std::fs::write(session_dir.join("marker"), b"materialized").unwrap();

    drop(runtime);

    assert!(!session_dir.exists());
    if cache_root.exists() {
        std::fs::remove_dir_all(cache_root).unwrap();
    }
}

#[test]
fn downloaded_response_is_queued_once_and_updates_canonical_state_after_send() {
    let mut world = WorldStore::new();
    let id = Uuid::from_u128(1);
    let key = push_accepted(&mut world, id, "https://example.test/server-pack.zip");
    let mut runtime = test_runtime("response");
    runtime.reconcile_world(&world, true);
    runtime.apply_task_event(PackTaskEvent::Downloaded { key });
    runtime.resolve_download_batches();
    let (commands, mut command_rx) = mpsc::channel(1);

    runtime.flush_responses(&mut world, &Some(commands));

    assert_eq!(
        command_rx.try_recv().unwrap(),
        NetCommand::ResourcePackResponse {
            id,
            action: ResourcePackResponseAction::Downloaded,
        }
    );
    assert_eq!(
        world
            .resource_pack(id)
            .unwrap()
            .last_response
            .unwrap()
            .action,
        ResourcePackResponseAction::Downloaded
    );
    assert!(runtime.responses.is_empty());
}

#[test]
fn resource_pack_prompt_shows_server_message_and_sends_the_players_choice() {
    use bbb_client_world::{DialogBodyState, DialogDataState};
    let mut world = WorldStore::new();
    let mut runtime = test_runtime("pack-consent");
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = crate::input::ClientInputState::new(true);
    let mut counters = crate::control::NetCounters::default();
    for number in 1..=2 {
        let id = Uuid::from_u128(number);
        world.apply_resource_pack_push(ResourcePackPush {
            id,
            url: "https://example.test/pack.zip".into(),
            hash: String::new(),
            required: number == 2,
            prompt: Some(bbb_protocol::RichTextComponent::plain(format!(
                "Pack {number}"
            ))),
        });
        world.request_resource_pack_consent(id);
    }
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    runtime.synchronize(&mut world, &commands, true);
    assert!(rx.try_recv().is_err());
    let DialogDataState::Confirmation { common, .. } =
        input.current_dialog().unwrap().data.clone().unwrap()
    else {
        panic!()
    };
    let DialogBodyState::PlainMessage { message } = &common.body_data[0] else {
        panic!()
    };
    assert!(message.contents.text.contains("Pack 2"));
    assert!(crate::runtime::handle_dialog_mouse_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        None,
        winit::event::MouseButton::Left,
        winit::event::ElementState::Pressed,
        Some(winit::dpi::PhysicalPosition::new(85.0, 220.0)),
        winit::dpi::PhysicalSize::new(320, 240),
    ));
    assert!(input.current_dialog().is_none());
    for number in 1..=2 {
        runtime.synchronize(&mut world, &commands, true);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ResourcePackResponse {
                id: Uuid::from_u128(number),
                action: ResourcePackResponseAction::Accepted,
            }
        );
        // Accepting a prompt must start tracking/batching the pack on the same step.
        assert!(runtime
            .packs
            .iter()
            .any(|pack| pack.key.id == Uuid::from_u128(number)));
    }
}

#[test]
fn required_decline_queues_one_local_disconnect() {
    let mut world = WorldStore::new();
    let id = Uuid::from_u128(1);
    world.apply_resource_pack_push(ResourcePackPush {
        id,
        url: "https://example.test/required.zip".to_string(),
        hash: String::new(),
        required: true,
        prompt: Some(bbb_protocol::RichTextComponent::plain("Required pack")),
    });
    world.apply_resource_pack_response(id, ResourcePackResponseAction::Declined);
    let mut runtime = test_runtime("required-decline");
    let (commands, mut command_rx) = mpsc::channel(2);

    runtime.disconnect_for_required_declines(&world, &Some(commands.clone()), true);
    runtime.disconnect_for_required_declines(&world, &Some(commands), true);

    assert!(
        matches!(command_rx.try_recv().unwrap(), NetCommand::DisconnectWithReason(reason) if reason.contains("requires its resource pack") && reason.contains("--server-resource-pack-policy accept"))
    );
    assert!(command_rx.try_recv().is_err());
}

#[test]
fn popped_pack_ignores_late_materialization_and_removes_its_root() {
    let mut world = WorldStore::new();
    let id = Uuid::from_u128(1);
    let key = push_accepted(&mut world, id, "https://example.test/server-pack.zip");
    let mut runtime = test_runtime("pop");
    runtime.reconcile_world(&world, true);
    world.apply_resource_pack_pop(ResourcePackPop { id: Some(id) });
    runtime.reconcile_world(&world, true);
    let root = runtime.materialized_session_dir.join("late");
    std::fs::create_dir_all(&root).unwrap();

    runtime.apply_task_event(PackTaskEvent::Ready {
        key,
        materialization: ready_materialization(root.clone()),
    });
    runtime.resolve_download_batches();
    runtime.cleanup_removed_packs();

    assert!(!root.exists());
    assert!(runtime.packs.is_empty());
    assert!(runtime.responses.is_empty());
}

#[test]
fn replaced_uuid_drops_old_result_without_mutating_new_request() {
    let mut world = WorldStore::new();
    let id = Uuid::from_u128(1);
    let old = push_accepted(&mut world, id, "https://example.test/old.zip");
    let mut runtime = test_runtime("replace");
    runtime.reconcile_world(&world, true);
    let new = push_accepted(&mut world, id, "https://example.test/new.zip");
    runtime.reconcile_world(&world, true);
    let stale_root = runtime.materialized_session_dir.join("stale");
    std::fs::create_dir_all(&stale_root).unwrap();

    runtime.apply_task_event(PackTaskEvent::Ready {
        key: old,
        materialization: ready_materialization(stale_root.clone()),
    });
    runtime.cleanup_removed_packs();

    assert!(!stale_root.exists());
    assert_eq!(runtime.packs.len(), 1);
    assert_eq!(runtime.packs[0].key, new);
    assert!(matches!(runtime.packs[0].state, PackDownloadState::Pending));
}

#[test]
fn ready_pack_reload_reports_success_only_after_barrier_completion() {
    let mut world = WorldStore::new();
    let id = Uuid::from_u128(1);
    let key = push_accepted(&mut world, id, "https://example.test/server-pack.zip");
    let mut runtime = test_runtime("reload-success");
    runtime.reconcile_world(&world, true);
    let root = ready_root(&runtime, "ready");
    runtime.apply_task_event(PackTaskEvent::Downloaded { key });
    runtime.apply_task_event(PackTaskEvent::Ready {
        key,
        materialization: ready_materialization(root.clone()),
    });
    runtime.resolve_download_batches();

    let reload = runtime.take_reload().unwrap();
    assert_eq!(selected_roots(&reload), vec![ready_archive_path(&root)]);
    assert!(runtime
        .responses
        .iter()
        .all(|response| { response.action != ResourcePackResponseAction::SuccessfullyLoaded }));
    assert_eq!(
        runtime
            .finish_reload(reload.generation, || Ok(()))
            .map(|(finish, _)| finish),
        Some(ResourcePackReloadFinish::Accepted)
    );
    assert!(runtime.packs[0].active);
    assert_eq!(
        runtime
            .responses
            .iter()
            .map(|response| response.action)
            .collect::<Vec<_>>(),
        vec![
            ResourcePackResponseAction::Downloaded,
            ResourcePackResponseAction::SuccessfullyLoaded,
        ]
    );
}

#[test]
fn web_user_pack_registry_rejection_preserves_the_active_server_stack() {
    let mut world = WorldStore::new();
    let mut runtime = test_runtime("local-reload");
    let id = Uuid::from_u128(1);
    let key = push_accepted(&mut world, id, "https://example.test/server-pack.zip");
    runtime.reconcile_world(&world, true);
    let root = ready_root(&runtime, "ready");
    runtime.apply_task_event(PackTaskEvent::Downloaded { key });
    runtime.apply_task_event(PackTaskEvent::Ready {
        key,
        materialization: ready_materialization(root.clone()),
    });
    runtime.resolve_download_batches();
    runtime.request_local_pack_reload();

    let server = runtime.take_reload().unwrap();
    assert!(!server.local_pack_change);
    runtime
        .finish_reload(server.generation, || Ok(()))
        .unwrap()
        .1
        .unwrap();
    runtime.responses.clear();

    let local = runtime.take_reload().unwrap();
    assert!(local.local_pack_change);
    assert_eq!(selected_roots(&local), vec![ready_archive_path(&root)]);
    let (_, result) = runtime
        .finish_reload(local.generation, || {
            Err::<(), _>("invalid local pack".into())
        })
        .unwrap();
    assert!(result.is_err());
    assert_eq!(runtime.packs[0].active_generation, Some(server.generation));
    assert!(runtime.packs[0].runtime_state.failure.is_none());
    assert!(runtime.responses.is_empty());
    assert!(runtime.take_reload().is_none());
}

#[test]
fn stale_reload_ack_does_not_commit_or_discard_the_current_barrier() {
    use bbb_protocol::{
        data_component_types::VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
        packets::ItemStackSummary,
    };
    use std::collections::BTreeMap;

    let mut world = WorldStore::new();
    let stack = ItemStackSummary {
        item_id: Some(7),
        count: 1,
        component_patch: Default::default(),
    };
    let apply = |world: &mut WorldStore| {
        world
            .replace_item_component_defaults(
                BTreeMap::from([(
                    7,
                    BTreeMap::from([(
                        VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
                        serde_json::json!(1),
                    )]),
                )]),
                0,
                Vec::new(),
            )
            .map_err(|error| error.to_string())
    };
    let id = Uuid::from_u128(1);
    let key = push_accepted(&mut world, id, "https://example.test/server-pack.zip");
    let mut runtime = test_runtime("stale-reload-ack");
    runtime.reconcile_world(&world, true);
    let root = ready_root(&runtime, "ready");
    runtime.apply_task_event(PackTaskEvent::Downloaded { key });
    runtime.apply_task_event(PackTaskEvent::Ready {
        key,
        materialization: ready_materialization(root),
    });
    runtime.resolve_download_batches();

    let reload = runtime.take_reload().unwrap();
    assert_eq!(
        runtime
            .finish_reload(reload.generation + 1, || apply(&mut world))
            .map(|(finish, _)| finish),
        None
    );
    assert_eq!(
        world
            .effective_item_stack(&stack)
            .component_patch
            .max_stack_size,
        None
    );
    assert!(!runtime.packs[0].active);
    assert!(runtime.take_reload().is_none());
    assert!(runtime
        .responses
        .iter()
        .all(|response| { response.action != ResourcePackResponseAction::SuccessfullyLoaded }));

    assert_eq!(
        runtime
            .finish_reload(reload.generation, || apply(&mut world))
            .map(|(finish, _)| finish),
        Some(ResourcePackReloadFinish::Accepted)
    );
    assert!(runtime.packs[0].active);
    assert_eq!(
        world
            .effective_item_stack(&stack)
            .component_patch
            .max_stack_size,
        Some(1)
    );
}

#[test]
fn accepted_pack_runs_download_apply_pop_restore_lifecycle() {
    let mut world = WorldStore::new();
    let id = Uuid::from_u128(1);
    let key = push_accepted(&mut world, id, "https://example.test/server-pack.zip");
    let mut runtime = test_runtime("full-lifecycle");
    runtime.reconcile_world(&world, true);
    let root = ready_root(&runtime, "ready");
    runtime.apply_task_event(PackTaskEvent::Downloaded { key });
    runtime.apply_task_event(PackTaskEvent::Ready {
        key,
        materialization: ready_materialization(root.clone()),
    });
    runtime.resolve_download_batches();
    runtime.publish_runtime_diagnostics(&mut world);
    assert_eq!(
        world.resource_pack(id).unwrap().runtime.phase,
        ResourcePackRuntimePhase::Ready
    );

    let (commands, mut command_rx) = mpsc::channel(4);
    runtime.flush_responses(&mut world, &Some(commands.clone()));
    assert_eq!(
        command_rx.try_recv().unwrap(),
        NetCommand::ResourcePackResponse {
            id,
            action: ResourcePackResponseAction::Downloaded,
        }
    );

    let load = runtime.take_reload().unwrap();
    runtime.publish_runtime_diagnostics(&mut world);
    let applying = &world.resource_pack(id).unwrap().runtime;
    assert_eq!(applying.phase, ResourcePackRuntimePhase::Applying);
    assert_eq!(applying.generation, Some(load.generation));
    assert_eq!(
        runtime
            .finish_reload(load.generation, || Ok(()))
            .map(|(finish, _)| finish),
        Some(ResourcePackReloadFinish::Accepted)
    );
    runtime.publish_runtime_diagnostics(&mut world);
    runtime.flush_responses(&mut world, &Some(commands));
    assert_eq!(
        command_rx.try_recv().unwrap(),
        NetCommand::ResourcePackResponse {
            id,
            action: ResourcePackResponseAction::SuccessfullyLoaded,
        }
    );
    let active = &world.resource_pack(id).unwrap().runtime;
    assert_eq!(active.phase, ResourcePackRuntimePhase::Active);
    assert_eq!(active.generation, Some(load.generation));
    assert_eq!(active.failure, None);

    world.apply_resource_pack_pop(ResourcePackPop { id: Some(id) });
    runtime.reconcile_world(&world, true);
    let unload = runtime.take_reload().unwrap();
    assert!(unload.server_packs.is_empty());
    assert_eq!(
        runtime
            .finish_reload(unload.generation, || Ok(()))
            .map(|(finish, _)| finish),
        Some(ResourcePackReloadFinish::Accepted)
    );
    runtime.cleanup_removed_packs();
    assert!(!root.exists());
    assert!(runtime.packs.is_empty());
    assert!(command_rx.try_recv().is_err());
}

#[test]
fn transfer_keeps_active_pack_while_terminal_disconnect_unloads_it() {
    let mut world = WorldStore::new();
    let id = Uuid::from_u128(1);
    let key = push_accepted(&mut world, id, "https://example.test/server-pack.zip");
    let mut runtime = test_runtime("session-replacement");
    runtime.reconcile_world(&world, true);
    let root = ready_root(&runtime, "active");
    runtime.apply_task_event(PackTaskEvent::Downloaded { key });
    runtime.apply_task_event(PackTaskEvent::Ready {
        key,
        materialization: ready_materialization(root.clone()),
    });
    runtime.resolve_download_batches();
    let load = runtime.take_reload().unwrap();
    assert_eq!(
        runtime
            .finish_reload(load.generation, || Ok(()))
            .map(|(finish, _)| finish),
        Some(ResourcePackReloadFinish::Accepted)
    );
    runtime.responses.clear();

    world.apply_transfer(bbb_protocol::packets::Transfer {
        host: "next.example.com".into(),
        port: 25566,
    });
    world.begin_connection_session(true);
    runtime.reconcile_world(&world, true);
    assert!(runtime.take_reload().is_none());
    assert!(root.exists());
    assert!(runtime.packs[0].active);

    world.end_connection_session();
    runtime.reconcile_world(&world, false);
    let unload = runtime.take_reload().unwrap();
    assert!(unload.server_packs.is_empty());
    assert_eq!(
        runtime
            .finish_reload(unload.generation, || Ok(()))
            .map(|(finish, _)| finish),
        Some(ResourcePackReloadFinish::Accepted)
    );
    assert!(!root.exists());
    assert!(runtime.packs.is_empty());
}

#[test]
fn active_pop_keeps_root_until_unload_reload_succeeds() {
    let mut world = WorldStore::new();
    let id = Uuid::from_u128(1);
    let key = push_accepted(&mut world, id, "https://example.test/server-pack.zip");
    let mut runtime = test_runtime("reload-pop");
    runtime.reconcile_world(&world, true);
    let root = ready_root(&runtime, "active");
    runtime.apply_task_event(PackTaskEvent::Downloaded { key });
    runtime.apply_task_event(PackTaskEvent::Ready {
        key,
        materialization: ready_materialization(root.clone()),
    });
    runtime.resolve_download_batches();
    let load = runtime.take_reload().unwrap();
    assert_eq!(
        runtime
            .finish_reload(load.generation, || Ok(()))
            .map(|(finish, _)| finish),
        Some(ResourcePackReloadFinish::Accepted)
    );
    runtime.responses.clear();

    world.apply_resource_pack_pop(ResourcePackPop { id: Some(id) });
    runtime.reconcile_world(&world, true);
    runtime.cleanup_removed_packs();
    assert!(root.exists());
    let unload = runtime.take_reload().unwrap();
    assert!(unload.server_packs.is_empty());

    assert_eq!(
        runtime
            .finish_reload(unload.generation, || Ok(()))
            .map(|(finish, _)| finish),
        Some(ResourcePackReloadFinish::Accepted)
    );
    assert!(!root.exists());
    assert!(runtime.packs.is_empty());
}

#[test]
fn recovery_reload_runs_once_and_second_failure_does_not_hot_loop() {
    let mut world = WorldStore::new();
    let id = Uuid::from_u128(1);
    let key = push_accepted(&mut world, id, "https://example.test/server-pack.zip");
    let mut runtime = test_runtime("bounded-recovery");
    runtime.reconcile_world(&world, true);
    let root = ready_root(&runtime, "active");
    runtime.apply_task_event(PackTaskEvent::Downloaded { key });
    runtime.apply_task_event(PackTaskEvent::Ready {
        key,
        materialization: ready_materialization(root.clone()),
    });
    runtime.resolve_download_batches();
    let load = runtime.take_reload().unwrap();
    assert_eq!(
        runtime
            .finish_reload(load.generation, || Ok(()))
            .map(|(finish, _)| finish),
        Some(ResourcePackReloadFinish::Accepted)
    );
    runtime.responses.clear();

    world.apply_resource_pack_pop(ResourcePackPop { id: Some(id) });
    runtime.reconcile_world(&world, true);
    let unload = runtime.take_reload().unwrap();
    assert!(unload.server_packs.is_empty());
    assert!(!runtime.reload.as_ref().unwrap().is_recovery);
    assert_eq!(
        runtime
            .finish_reload(unload.generation, || Err::<(), _>(
                "first failure".to_string()
            ))
            .map(|(finish, _)| finish),
        Some(ResourcePackReloadFinish::Accepted)
    );
    assert!(root.exists());

    let recovery = runtime.take_reload().expect("one recovery reload");
    assert!(recovery.server_packs.is_empty());
    assert!(runtime.reload.as_ref().unwrap().is_recovery);
    assert_eq!(
        runtime
            .finish_reload(recovery.generation, || Err::<(), _>(
                "recovery failure".to_string()
            ))
            .map(|(finish, _)| finish),
        Some(ResourcePackReloadFinish::RecoveryFailed)
    );
    assert!(root.exists());
    assert!(runtime.take_reload().is_none());

    runtime.request_reload();
    let explicit_retry = runtime.take_reload().expect("explicit retry");
    assert!(!runtime.reload.as_ref().unwrap().is_recovery);
    assert_eq!(
        runtime
            .finish_reload(explicit_retry.generation, || Ok(()))
            .map(|(finish, _)| finish),
        Some(ResourcePackReloadFinish::Accepted)
    );
    assert!(!root.exists());
    assert!(runtime.packs.is_empty());
}

#[test]
fn failed_reload_keeps_previous_active_stack_and_fails_only_new_pack() {
    let mut world = WorldStore::new();
    let old_id = Uuid::from_u128(1);
    let old_key = push_accepted(&mut world, old_id, "https://example.test/old.zip");
    let mut runtime = test_runtime("reload-failure");
    runtime.reconcile_world(&world, true);
    let old_root = ready_root(&runtime, "old");
    runtime.apply_task_event(PackTaskEvent::Downloaded { key: old_key });
    runtime.apply_task_event(PackTaskEvent::Ready {
        key: old_key,
        materialization: ready_materialization(old_root.clone()),
    });
    runtime.resolve_download_batches();
    let load_old = runtime.take_reload().unwrap();
    assert_eq!(
        runtime
            .finish_reload(load_old.generation, || Ok(()))
            .map(|(finish, _)| finish),
        Some(ResourcePackReloadFinish::Accepted)
    );
    runtime.responses.clear();

    let new_id = Uuid::from_u128(2);
    let new_key = push_accepted(&mut world, new_id, "https://example.test/new.zip");
    runtime.reconcile_world(&world, true);
    let new_root = ready_root(&runtime, "new");
    runtime.apply_task_event(PackTaskEvent::Downloaded { key: new_key });
    runtime.apply_task_event(PackTaskEvent::Ready {
        key: new_key,
        materialization: ready_materialization(new_root.clone()),
    });
    runtime.resolve_download_batches();
    let reload = runtime.take_reload().unwrap();
    assert_eq!(
        selected_roots(&reload),
        vec![ready_archive_path(&new_root), ready_archive_path(&old_root),]
    );
    assert_eq!(
        selected_ids(&reload),
        vec![
            format!("server/00000001/{new_id}"),
            format!("server/00000002/{}", old_key.id),
        ]
    );

    assert_eq!(
        runtime
            .finish_reload(reload.generation, || Err::<(), _>(
                "reload failed".to_string()
            ))
            .map(|(finish, _)| finish),
        Some(ResourcePackReloadFinish::Accepted)
    );
    runtime.publish_runtime_diagnostics(&mut world);
    assert!(old_root.exists());
    assert!(!new_root.exists());
    assert_eq!(
        runtime
            .responses
            .iter()
            .map(|response| response.action)
            .collect::<Vec<_>>(),
        vec![
            ResourcePackResponseAction::Downloaded,
            ResourcePackResponseAction::FailedReload,
        ]
    );
    assert_eq!(runtime.packs.len(), 2);
    assert!(runtime
        .packs
        .iter()
        .any(|pack| pack.key == old_key && pack.active));
    assert_eq!(
        world.resource_pack(old_id).unwrap().runtime,
        ResourcePackRuntimeState {
            phase: ResourcePackRuntimePhase::Active,
            generation: Some(load_old.generation),
            failure: Some(ResourcePackRuntimeFailure {
                stage: ResourcePackFailureStage::Reload,
                generation: Some(reload.generation),
                error: "reload failed".to_string(),
            }),
        }
    );
    let failed = &world.resource_pack(new_id).unwrap().runtime;
    assert_eq!(failed.phase, ResourcePackRuntimePhase::Failed);
    assert_eq!(failed.generation, Some(reload.generation));
    assert_eq!(
        failed.failure.as_ref().unwrap().stage,
        ResourcePackFailureStage::Reload
    );
    assert_eq!(
        failed.failure.as_ref().unwrap().generation,
        Some(reload.generation)
    );
    assert_eq!(failed.failure.as_ref().unwrap().error, "reload failed");
    runtime.responses.clear();
    runtime.cleanup_removed_packs();
    assert_eq!(runtime.packs.len(), 1);
    assert_eq!(runtime.packs[0].key, old_key);
    let recovery = runtime.take_reload().expect("fallback recovery reload");
    assert_eq!(
        selected_roots(&recovery),
        vec![ready_archive_path(&old_root)]
    );
    assert_eq!(
        selected_ids(&recovery),
        vec![format!("server/00000003/{}", old_key.id)]
    );
    assert!(runtime.reload.as_ref().unwrap().is_recovery);
    assert_eq!(
        runtime
            .finish_reload(recovery.generation, || Ok(()))
            .map(|(finish, _)| finish),
        Some(ResourcePackReloadFinish::RecoverySucceeded)
    );
    assert!(runtime.take_reload().is_none());
}

#[test]
fn explicit_reload_rebuilds_unchanged_active_stack_once() {
    let mut world = WorldStore::new();
    let id = Uuid::from_u128(1);
    let key = push_accepted(&mut world, id, "https://example.test/server-pack.zip");
    let mut runtime = test_runtime("explicit-reload");
    runtime.reconcile_world(&world, true);
    let root = ready_root(&runtime, "active");
    runtime.apply_task_event(PackTaskEvent::Downloaded { key });
    runtime.apply_task_event(PackTaskEvent::Ready {
        key,
        materialization: ready_materialization(root.clone()),
    });
    runtime.resolve_download_batches();
    let initial = runtime.take_reload().unwrap();
    assert_eq!(
        runtime
            .finish_reload(initial.generation, || Ok(()))
            .map(|(finish, _)| finish),
        Some(ResourcePackReloadFinish::Accepted)
    );
    runtime.responses.clear();
    assert!(runtime.take_reload().is_none());

    runtime.request_reload();
    let forced = runtime.take_reload().unwrap();
    assert_eq!(selected_roots(&forced), vec![ready_archive_path(&root)]);
    assert_eq!(
        runtime
            .finish_reload(forced.generation, || Ok(()))
            .map(|(finish, _)| finish),
        Some(ResourcePackReloadFinish::Accepted)
    );
    assert!(runtime.responses.is_empty());
    assert!(runtime.take_reload().is_none());
}

#[test]
fn one_download_failure_discards_every_other_inactive_pack_across_queued_batches() {
    let mut world = WorldStore::new();
    let first = push_accepted(
        &mut world,
        Uuid::from_u128(1),
        "https://example.test/first.zip",
    );
    let mut runtime = test_runtime("batch-failure");
    runtime.reconcile_world(&world, true);
    assert_eq!(runtime.download_batches.len(), 1);

    let second = push_accepted(
        &mut world,
        Uuid::from_u128(2),
        "https://example.test/second.zip",
    );
    runtime.reconcile_world(&world, true);
    assert_eq!(runtime.download_batches.len(), 2);
    assert_ne!(
        runtime.download_batches[0].id,
        runtime.download_batches[1].id
    );

    runtime.apply_task_event(PackTaskEvent::Failed {
        key: first,
        action: ResourcePackResponseAction::FailedDownload,
        error: "connection reset".to_string(),
    });
    runtime.resolve_download_batches();
    runtime.cleanup_removed_packs();

    assert_eq!(runtime.packs.len(), 2);
    assert!(runtime.packs.iter().all(|pack| pack.removed));
    assert_eq!(
        runtime
            .responses
            .iter()
            .map(|response| (response.key, response.action))
            .collect::<Vec<_>>(),
        vec![
            (first, ResourcePackResponseAction::FailedDownload),
            (second, ResourcePackResponseAction::Discarded),
        ]
    );
    runtime.reconcile_world(&world, true);
    assert_eq!(runtime.packs.len(), 2);
    runtime.responses.clear();
    runtime.cleanup_removed_packs();
    assert!(runtime.packs.is_empty());
    assert!(runtime.take_reload().is_none());
}

#[test]
fn one_preparation_failure_fails_every_new_pack_across_queued_batches() {
    let mut world = WorldStore::new();
    let first = push_accepted(
        &mut world,
        Uuid::from_u128(1),
        "https://example.test/first.zip",
    );
    let mut runtime = test_runtime("preparation-failure");
    runtime.reconcile_world(&world, true);

    let second = push_accepted(
        &mut world,
        Uuid::from_u128(2),
        "https://example.test/second.zip",
    );
    runtime.reconcile_world(&world, true);
    assert_eq!(runtime.download_batches.len(), 2);

    runtime.apply_task_event(PackTaskEvent::Downloaded { key: first });
    runtime.apply_task_event(PackTaskEvent::Failed {
        key: first,
        action: ResourcePackResponseAction::FailedReload,
        error: "invalid pack metadata".to_string(),
    });
    runtime.resolve_download_batches();
    runtime.resolve_preparation_failures();
    assert!(!runtime.packs[0].removed);
    assert!(!runtime.packs[1].removed);

    let second_root = ready_root(&runtime, "second");
    runtime.apply_task_event(PackTaskEvent::Downloaded { key: second });
    runtime.apply_task_event(PackTaskEvent::Ready {
        key: second,
        materialization: ready_materialization(second_root.clone()),
    });
    runtime.resolve_download_batches();
    runtime.resolve_preparation_failures();
    runtime.cleanup_removed_packs();

    assert!(!second_root.exists());
    assert_eq!(runtime.packs.len(), 2);
    assert!(runtime.packs.iter().all(|pack| pack.removed));
    assert_eq!(
        runtime
            .responses
            .iter()
            .map(|response| (response.key, response.action))
            .collect::<Vec<_>>(),
        vec![
            (first, ResourcePackResponseAction::Downloaded),
            (second, ResourcePackResponseAction::Downloaded),
            (first, ResourcePackResponseAction::FailedReload),
            (second, ResourcePackResponseAction::FailedReload),
        ]
    );
    runtime.reconcile_world(&world, true);
    assert_eq!(runtime.packs.len(), 2);
    runtime.responses.clear();
    runtime.cleanup_removed_packs();
    assert!(runtime.packs.is_empty());
    assert!(runtime.take_reload().is_none());
}

#[test]
fn popped_and_dropped_virtual_server_packs_release_virtual_store_archives() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let download_dir = PathBuf::from(format!(
        "/__bbb_mem__/server-packs-test-{}-{nonce}",
        std::process::id()
    ));
    let mut runtime = ServerResourcePackRuntime::with_optional_runtime(
        None,
        download_dir.clone(),
        "bbb-client".to_string(),
        Uuid::nil(),
        None,
    );
    let mut world = WorldStore::new();
    let first_id = Uuid::from_u128(0x101);
    let second_id = Uuid::from_u128(0x102);
    let shared_id = Uuid::from_u128(0x103);
    let first_key = push_accepted(&mut world, first_id, "https://example.com/a.zip");
    let second_key = push_accepted(&mut world, second_id, "https://example.com/b.zip");
    let shared_key = push_accepted(&mut world, shared_id, "https://example.com/b-dup.zip");
    runtime.reconcile_world(&world, true);

    let make_zip_bytes = || -> Arc<[u8]> {
        let mut cursor = std::io::Cursor::new(Vec::new());
        {
            let mut archive = ZipWriter::new(&mut cursor);
            archive
                .start_file(
                    "pack.mcmeta",
                    SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
                )
                .unwrap();
            archive
                .write_all(
                    br#"{"pack":{"description":"vtest","min_format":[84,0],"max_format":[84,0]}}"#,
                )
                .unwrap();
            archive.finish().unwrap();
        }
        Arc::from(cursor.into_inner())
    };

    let archive_a = download_dir.join("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    let archive_b = download_dir.join("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    bbb_client_assets::insert_virtual_file(&archive_a, make_zip_bytes());
    bbb_client_assets::insert_virtual_file(&archive_b, make_zip_bytes());
    assert!(bbb_client_assets::has_virtual_file(&archive_a));
    assert!(bbb_client_assets::has_virtual_file(&archive_b));

    let root_a = download_dir.join("cache-a");
    let root_b = download_dir.join("cache-b");
    let root_c = download_dir.join("cache-c");
    runtime.apply_task_event(PackTaskEvent::Ready {
        key: first_key,
        materialization: prepare_pack_archive(&archive_a, &root_a).unwrap(),
    });
    runtime.apply_task_event(PackTaskEvent::Ready {
        key: second_key,
        materialization: prepare_pack_archive(&archive_b, &root_b).unwrap(),
    });
    runtime.apply_task_event(PackTaskEvent::Ready {
        key: shared_key,
        materialization: prepare_pack_archive(&archive_b, &root_c).unwrap(),
    });

    // Pop `first_id` (unique archive A) and `second_id` (archive B still referenced by `shared_id`).
    world.apply_resource_pack_pop(ResourcePackPop { id: Some(first_id) });
    world.apply_resource_pack_pop(ResourcePackPop {
        id: Some(second_id),
    });
    runtime.reconcile_world(&world, true);
    runtime.cleanup_removed_packs();

    assert!(
        !bbb_client_assets::has_virtual_file(&archive_a),
        "popped pack archive must be evicted from virtual_store"
    );
    assert!(
        bbb_client_assets::has_virtual_file(&archive_b),
        "shared archive still referenced by another pack must remain in virtual_store"
    );

    // Dropping the runtime releases any remaining downloaded virtual archives.
    drop(runtime);
    assert!(
        !bbb_client_assets::has_virtual_file(&archive_b),
        "Drop must release all remaining virtual server-pack archives"
    );
}

fn ready_root(runtime: &ServerResourcePackRuntime, name: &str) -> PathBuf {
    let root = runtime.materialized_session_dir.join(name);
    std::fs::create_dir_all(&root).unwrap();
    root
}

fn ready_materialization(root: PathBuf) -> PreparedPackArchive {
    let archive_path = ready_archive_path(&root);
    let file = File::create(&archive_path).unwrap();
    let mut archive = ZipWriter::new(file);
    archive
        .start_file(
            "pack.mcmeta",
            SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
        )
        .unwrap();
    archive
        .write_all(br#"{"pack":{"description":"test","min_format":[84,0],"max_format":[84,0]}}"#)
        .unwrap();
    archive.finish().unwrap();
    prepare_pack_archive(&archive_path, &root).unwrap()
}

fn ready_archive_path(root: &Path) -> PathBuf {
    root.with_extension("zip")
}

fn selected_roots(reload: &ServerResourcePackReload) -> Vec<PathBuf> {
    reload
        .server_packs
        .iter()
        .map(|pack| pack.root.clone())
        .collect()
}

fn selected_ids(reload: &ServerResourcePackReload) -> Vec<String> {
    reload
        .server_packs
        .iter()
        .map(|pack| pack.id.clone())
        .collect()
}

fn unique_temp_dir(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "bbb-client-server-pack-runtime-{label}-{}-{nonce}",
        std::process::id()
    ))
}
