//! Dedicated shared-memory Web Worker running `bbb-client-net` (WebSocket I/O,
//! zlib inflate/deflate, and packet decode/encode) off the browser `Window` thread.

use std::{
    cell::RefCell,
    future::poll_fn,
    rc::Rc,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    task::{Poll, Waker},
};

use bbb_client_net::{ConnectionOptions, EventStreamExit, NetCommand, NetEvent, NetEventSender};
use bbb_render_types::wasm_sync::{
    lock_free_channel, AtomicWorkStack, LockFreeReceiver, LockFreeSender,
};
use tokio::sync::{mpsc, oneshot};
use wasm_bindgen::{closure::Closure, JsCast, JsValue};

use crate::{
    network_runtime::{NETWORK_COMMAND_CAPACITY, WEB_NETWORK_EVENT_CAPACITY},
    simulation::commands::LoopWake,
};

struct SharedSessionControl {
    activated: AtomicBool,
    closed: AtomicBool,
    stream_done: AtomicBool,
    finished: AtomicBool,
    in_flight_events: AtomicUsize,
    worker_notify_pending: AtomicBool,
    main_notify_pending: AtomicBool,
}

impl SharedSessionControl {
    fn new() -> Self {
        Self {
            activated: AtomicBool::new(false),
            closed: AtomicBool::new(false),
            stream_done: AtomicBool::new(false),
            finished: AtomicBool::new(false),
            in_flight_events: AtomicUsize::new(0),
            worker_notify_pending: AtomicBool::new(false),
            main_notify_pending: AtomicBool::new(false),
        }
    }
}

struct WorkerSessionJob {
    options: ConnectionOptions,
    control: Arc<SharedSessionControl>,
    cmd_rx: LockFreeReceiver<NetCommand>,
    evt_tx: LockFreeSender<NetEvent>,
}

static PENDING_JOBS: AtomicWorkStack<WorkerSessionJob> = AtomicWorkStack::new();

struct MainSessionPump {
    control: Arc<SharedSessionControl>,
    evt_rx: LockFreeReceiver<NetEvent>,
    pending_event: Option<NetEvent>,
    backpressure_waiting: bool,
    main_event_tx: mpsc::Sender<NetEvent>,
    wake: Arc<LoopWake>,
}

#[derive(Clone)]
struct NetWorkerHandle {
    worker: JsValue,
    post_fn: js_sys::Function,
    sessions: Rc<RefCell<Vec<Rc<RefCell<MainSessionPump>>>>>,
}

impl NetWorkerHandle {
    fn notify_worker(&self, control: &SharedSessionControl) {
        if !control.worker_notify_pending.swap(true, Ordering::SeqCst) {
            let _ = self.post_fn.call1(&self.worker, &JsValue::from_f64(1.0));
        }
    }

    fn force_post_worker(&self) {
        let _ = self.post_fn.call1(&self.worker, &JsValue::from_f64(1.0));
    }
}

thread_local! {
    static MAIN_NET_WORKER: RefCell<Option<NetWorkerHandle>> = const { RefCell::new(None) };
    static WORKER_POST_FN: RefCell<Option<(JsValue, js_sys::Function)>> = const { RefCell::new(None) };
    static WORKER_WAKERS: RefCell<Vec<Waker>> = const { RefCell::new(Vec::new()) };
}

fn resolve_pkg_url(window: &web_sys::Window) -> Option<String> {
    if let Ok(val) = js_sys::Reflect::get(window, &JsValue::from_str("__bbb_pkg_url")) {
        if let Some(url) = val.as_string().filter(|s| !s.is_empty()) {
            return Some(url);
        }
    }
    let base = window.location().href().ok()?;
    web_sys::Url::new_with_base("./pkg/bbb_client.js", &base)
        .ok()
        .map(|u| u.href())
}

pub(super) async fn prewarm_net_worker(window: &web_sys::Window) -> bool {
    if MAIN_NET_WORKER.with(|slot| slot.borrow().is_some()) {
        return true;
    }
    let Some(pkg_url) = resolve_pkg_url(window) else {
        return false;
    };
    let Ok(worker_ctor) = js_sys::Reflect::get(window, &JsValue::from_str("Worker"))
        .and_then(|v| v.dyn_into::<js_sys::Function>())
    else {
        return false;
    };

    let worker_source = r#"
        self.onmessage = async (ev) => {
            const d = ev.data;
            if (d && d.pkgUrl) {
                try {
                    const pkg = await import(d.pkgUrl);
                    await pkg.default({ module_or_path: d.module, memory: d.memory });
                    pkg.init_web_net_worker();
                    self.postMessage("ready");
                } catch (err) {
                    self.postMessage({ error: String(err) });
                }
            }
        };
    "#;
    let parts = js_sys::Array::of1(&JsValue::from_str(worker_source));
    let bag = web_sys::BlobPropertyBag::new();
    bag.set_type("text/javascript");
    let Ok(blob) = web_sys::Blob::new_with_str_sequence_and_options(&parts, &bag) else {
        return false;
    };
    let Ok(blob_url) = web_sys::Url::create_object_url_with_blob(&blob) else {
        return false;
    };

    let opts = js_sys::Object::new();
    let _ = js_sys::Reflect::set(
        &opts,
        &JsValue::from_str("type"),
        &JsValue::from_str("module"),
    );
    let ctor_args = js_sys::Array::of2(&JsValue::from_str(&blob_url), &opts);
    let Ok(worker) = js_sys::Reflect::construct(&worker_ctor, &ctor_args) else {
        let _ = web_sys::Url::revoke_object_url(&blob_url);
        return false;
    };
    let Ok(post_fn) = js_sys::Reflect::get(&worker, &JsValue::from_str("postMessage"))
        .and_then(|v| v.dyn_into::<js_sys::Function>())
    else {
        let _ = web_sys::Url::revoke_object_url(&blob_url);
        return false;
    };

    let (ready_tx, ready_rx) = oneshot::channel::<bool>();
    let ready_tx = Rc::new(RefCell::new(Some(ready_tx)));

    let ok_signal = Rc::clone(&ready_tx);
    let on_ready_msg =
        Closure::<dyn FnMut(web_sys::MessageEvent)>::new(move |ev: web_sys::MessageEvent| {
            let ready = ev.data().as_string().as_deref() == Some("ready");
            if let Some(tx) = ok_signal.borrow_mut().take() {
                let _ = tx.send(ready);
            }
        });
    let err_signal = Rc::clone(&ready_tx);
    let on_boot_err = Closure::<dyn FnMut(web_sys::Event)>::new(move |_ev: web_sys::Event| {
        if let Some(tx) = err_signal.borrow_mut().take() {
            let _ = tx.send(false);
        }
    });

    let _ = js_sys::Reflect::set(
        &worker,
        &JsValue::from_str("onmessage"),
        on_ready_msg.as_ref().unchecked_ref(),
    );
    let _ = js_sys::Reflect::set(
        &worker,
        &JsValue::from_str("onerror"),
        on_boot_err.as_ref().unchecked_ref(),
    );

    let init_msg = js_sys::Object::new();
    let _ = js_sys::Reflect::set(
        &init_msg,
        &JsValue::from_str("pkgUrl"),
        &JsValue::from_str(&pkg_url),
    );
    let _ = js_sys::Reflect::set(
        &init_msg,
        &JsValue::from_str("module"),
        &wasm_bindgen::module(),
    );
    let _ = js_sys::Reflect::set(
        &init_msg,
        &JsValue::from_str("memory"),
        &wasm_bindgen::memory(),
    );
    if post_fn.call1(&worker, &init_msg).is_err() {
        let _ = web_sys::Url::revoke_object_url(&blob_url);
        return false;
    }

    let booted = ready_rx.await.unwrap_or(false);
    let _ = web_sys::Url::revoke_object_url(&blob_url);
    drop(on_ready_msg);
    drop(on_boot_err);
    if !booted {
        let _ = js_sys::Reflect::set(&worker, &JsValue::from_str("onmessage"), &JsValue::NULL);
        let _ = js_sys::Reflect::set(&worker, &JsValue::from_str("onerror"), &JsValue::NULL);
        tracing::warn!("dedicated Web network Worker failed to initialize; falling back to Window");
        return false;
    }

    let sessions: Rc<RefCell<Vec<Rc<RefCell<MainSessionPump>>>>> =
        Rc::new(RefCell::new(Vec::new()));
    let handle = NetWorkerHandle {
        worker: worker.clone(),
        post_fn,
        sessions: Rc::clone(&sessions),
    };
    let handle_for_msg = handle.clone();
    let on_runtime_msg =
        Closure::<dyn FnMut(web_sys::MessageEvent)>::new(move |_ev: web_sys::MessageEvent| {
            pump_all_main_sessions(&handle_for_msg);
        });
    let _ = js_sys::Reflect::set(
        &worker,
        &JsValue::from_str("onmessage"),
        on_runtime_msg.as_ref().unchecked_ref(),
    );
    on_runtime_msg.forget();

    let on_runtime_err = Closure::<dyn FnMut(web_sys::Event)>::new(move |ev: web_sys::Event| {
        let msg = js_sys::Reflect::get(&ev, &JsValue::from_str("message"))
            .ok()
            .and_then(|v| v.as_string())
            .unwrap_or_else(|| "dedicated network Worker error".to_string());
        tracing::error!(error = %msg, "dedicated Web network Worker faulted");
    });
    let _ = js_sys::Reflect::set(
        &worker,
        &JsValue::from_str("onerror"),
        on_runtime_err.as_ref().unchecked_ref(),
    );
    on_runtime_err.forget();

    MAIN_NET_WORKER.with(|slot| *slot.borrow_mut() = Some(handle));
    true
}

fn prune_finished_sessions(handle: &NetWorkerHandle) {
    handle.sessions.borrow_mut().retain(|session| {
        let Ok(guard) = session.try_borrow() else {
            return true;
        };
        !(guard.control.finished.load(Ordering::Acquire)
            && guard.pending_event.is_none()
            && guard.control.in_flight_events.load(Ordering::Acquire) == 0)
    });
}

fn pump_all_main_sessions(handle: &NetWorkerHandle) {
    let active: Vec<Rc<RefCell<MainSessionPump>>> = handle.sessions.borrow().clone();
    for session in active {
        pump_one_main_session(handle, &session);
    }
    prune_finished_sessions(handle);
}

fn pump_one_main_session(handle: &NetWorkerHandle, session_rc: &Rc<RefCell<MainSessionPump>>) {
    let mut spawn_backpressure = false;
    {
        let mut session = session_rc.borrow_mut();
        session
            .control
            .main_notify_pending
            .store(false, Ordering::SeqCst);

        let mut delivered = false;
        let mut freed_count = 0usize;

        if let Some(pending) = session.pending_event.take() {
            match session.main_event_tx.try_send(pending) {
                Ok(()) => {
                    delivered = true;
                    freed_count += 1;
                }
                Err(mpsc::error::TrySendError::Full(returned)) => {
                    session.pending_event = Some(returned);
                    if !session.backpressure_waiting {
                        session.backpressure_waiting = true;
                        spawn_backpressure = true;
                    }
                }
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    session.control.closed.store(true, Ordering::SeqCst);
                    handle.notify_worker(&session.control);
                    drop(session);
                    prune_finished_sessions(handle);
                    return;
                }
            }
        }

        if session.pending_event.is_none() {
            while let Ok(event) = session.evt_rx.try_recv() {
                match session.main_event_tx.try_send(event) {
                    Ok(()) => {
                        delivered = true;
                        freed_count += 1;
                    }
                    Err(mpsc::error::TrySendError::Full(returned)) => {
                        session.pending_event = Some(returned);
                        if !session.backpressure_waiting {
                            session.backpressure_waiting = true;
                            spawn_backpressure = true;
                        }
                        break;
                    }
                    Err(mpsc::error::TrySendError::Closed(_)) => {
                        session.control.closed.store(true, Ordering::SeqCst);
                        handle.notify_worker(&session.control);
                        drop(session);
                        prune_finished_sessions(handle);
                        return;
                    }
                }
            }
        }

        if freed_count > 0 {
            let prev = session
                .control
                .in_flight_events
                .fetch_sub(freed_count, Ordering::SeqCst);
            if prev >= WEB_NETWORK_EVENT_CAPACITY {
                handle.notify_worker(&session.control);
            }
        }
        if delivered {
            session.wake.wake();
        }
    }

    if spawn_backpressure {
        let tx = session_rc.borrow().main_event_tx.clone();
        let handle_clone = handle.clone();
        let session_clone = Rc::clone(session_rc);
        wasm_bindgen_futures::spawn_local(async move {
            let permit_ok = tx.reserve().await.is_ok();
            session_clone.borrow_mut().backpressure_waiting = false;
            if permit_ok {
                pump_one_main_session(&handle_clone, &session_clone);
            } else {
                let ctrl = Arc::clone(&session_clone.borrow().control);
                ctrl.closed.store(true, Ordering::SeqCst);
                handle_clone.notify_worker(&ctrl);
                prune_finished_sessions(&handle_clone);
            }
        });
    } else {
        prune_finished_sessions(handle);
    }
}

pub(crate) fn try_start_web_session(
    options: ConnectionOptions,
    wake: Arc<LoopWake>,
) -> Option<(
    mpsc::Receiver<NetEvent>,
    mpsc::Sender<NetCommand>,
    oneshot::Sender<()>,
    oneshot::Sender<()>,
)> {
    let handle = MAIN_NET_WORKER.with(|slot| slot.borrow().clone())?;
    let control = Arc::new(SharedSessionControl::new());
    let (cmd_tx, cmd_rx) = lock_free_channel::<NetCommand>();
    let (evt_tx, evt_rx) = lock_free_channel::<NetEvent>();

    let (main_event_tx, main_event_rx) = mpsc::channel(WEB_NETWORK_EVENT_CAPACITY);
    let (main_cmd_tx, mut main_cmd_rx) = mpsc::channel(NETWORK_COMMAND_CAPACITY);
    let (connection_start, connection_start_rx) = oneshot::channel();
    let (terminal_close, terminal_close_rx) = oneshot::channel();

    let session_pump = Rc::new(RefCell::new(MainSessionPump {
        control: Arc::clone(&control),
        evt_rx,
        pending_event: None,
        backpressure_waiting: false,
        main_event_tx,
        wake,
    }));
    handle.sessions.borrow_mut().push(Rc::clone(&session_pump));

    PENDING_JOBS.push(WorkerSessionJob {
        options,
        control: Arc::clone(&control),
        cmd_rx,
        evt_tx,
    });
    handle.force_post_worker();

    let start_ctrl = Arc::clone(&control);
    let start_handle = handle.clone();
    wasm_bindgen_futures::spawn_local(async move {
        if connection_start_rx.await.is_ok() {
            start_ctrl.activated.store(true, Ordering::SeqCst);
        } else {
            start_ctrl.closed.store(true, Ordering::SeqCst);
        }
        start_handle.notify_worker(&start_ctrl);
    });

    let close_ctrl = Arc::clone(&control);
    let close_handle = handle.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let _ = terminal_close_rx.await;
        close_ctrl.closed.store(true, Ordering::SeqCst);
        close_handle.notify_worker(&close_ctrl);
    });

    let cmd_ctrl = Arc::clone(&control);
    let cmd_handle = handle;
    wasm_bindgen_futures::spawn_local(async move {
        while let Some(cmd) = main_cmd_rx.recv().await {
            if cmd_ctrl.closed.load(Ordering::Acquire) || cmd_tx.send(cmd).is_err() {
                break;
            }
            cmd_handle.notify_worker(&cmd_ctrl);
        }
        cmd_ctrl.closed.store(true, Ordering::SeqCst);
        cmd_handle.notify_worker(&cmd_ctrl);
    });

    Some((main_event_rx, main_cmd_tx, connection_start, terminal_close))
}

fn notify_main_from_worker(control: &SharedSessionControl) {
    if !control.main_notify_pending.swap(true, Ordering::SeqCst) {
        WORKER_POST_FN.with(|slot| {
            if let Some((global, post_fn)) = slot.borrow().as_ref() {
                let _ = post_fn.call1(global, &JsValue::from_f64(1.0));
            }
        });
    }
}

fn register_worker_waker(waker: &Waker) {
    WORKER_WAKERS.with(|slot| {
        let mut wakers = slot.borrow_mut();
        if !wakers.iter().any(|existing| existing.will_wake(waker)) {
            wakers.push(waker.clone());
        }
    });
}

fn wake_all_worker_tasks() {
    let wakers: Vec<Waker> = WORKER_WAKERS.with(|slot| std::mem::take(&mut *slot.borrow_mut()));
    for waker in wakers {
        waker.wake();
    }
}

pub(super) fn init_worker_scope() {
    let global = js_sys::global();
    if let Ok(post_fn) = js_sys::Reflect::get(&global, &JsValue::from_str("postMessage"))
        .and_then(|v| v.dyn_into::<js_sys::Function>())
    {
        WORKER_POST_FN.with(|slot| *slot.borrow_mut() = Some((global.clone().into(), post_fn)));
    }

    let on_msg =
        Closure::<dyn FnMut(web_sys::MessageEvent)>::new(move |_ev: web_sys::MessageEvent| {
            drain_worker_inbox();
        });
    let _ = js_sys::Reflect::set(
        &global,
        &JsValue::from_str("onmessage"),
        on_msg.as_ref().unchecked_ref(),
    );
    on_msg.forget();

    drain_worker_inbox();
}

fn drain_worker_inbox() {
    for job in PENDING_JOBS.drain_fifo() {
        spawn_worker_session(job);
    }
    wake_all_worker_tasks();
}

fn spawn_worker_session(job: WorkerSessionJob) {
    let WorkerSessionJob {
        options,
        control,
        mut cmd_rx,
        evt_tx,
    } = job;

    let (worker_event_tx, mut worker_event_rx) =
        mpsc::channel::<NetEvent>(WEB_NETWORK_EVENT_CAPACITY);
    let (worker_cmd_tx, worker_cmd_rx) = mpsc::channel::<NetCommand>(NETWORK_COMMAND_CAPACITY);

    let network_tx = NetEventSender::new(worker_event_tx, || {});
    let final_event_tx = network_tx.clone();

    let evt_ctrl = Arc::clone(&control);
    wasm_bindgen_futures::spawn_local(async move {
        while let Some(event) = worker_event_rx.recv().await {
            let can_send = poll_fn(|cx| {
                register_worker_waker(cx.waker());
                evt_ctrl
                    .worker_notify_pending
                    .store(false, Ordering::SeqCst);
                if evt_ctrl.closed.load(Ordering::Acquire) {
                    return Poll::Ready(false);
                }
                if evt_ctrl.in_flight_events.load(Ordering::Acquire) < WEB_NETWORK_EVENT_CAPACITY {
                    return Poll::Ready(true);
                }
                Poll::Pending
            })
            .await;
            if !can_send {
                break;
            }
            evt_ctrl.in_flight_events.fetch_add(1, Ordering::SeqCst);
            if evt_tx.send(event).is_err() {
                evt_ctrl.in_flight_events.fetch_sub(1, Ordering::SeqCst);
                break;
            }
            notify_main_from_worker(&evt_ctrl);
        }
        evt_ctrl.finished.store(true, Ordering::Release);
        notify_main_from_worker(&evt_ctrl);
    });

    let cmd_ctrl = Arc::clone(&control);
    wasm_bindgen_futures::spawn_local(async move {
        loop {
            let next_cmd = poll_fn(|cx| {
                register_worker_waker(cx.waker());
                cmd_ctrl
                    .worker_notify_pending
                    .store(false, Ordering::SeqCst);
                if cmd_ctrl.closed.load(Ordering::Acquire)
                    || cmd_ctrl.stream_done.load(Ordering::Acquire)
                {
                    return Poll::Ready(None);
                }
                if let Ok(cmd) = cmd_rx.try_recv() {
                    return Poll::Ready(Some(cmd));
                }
                Poll::Pending
            })
            .await;
            let Some(cmd) = next_cmd else {
                break;
            };
            if worker_cmd_tx.send(cmd).await.is_err() {
                break;
            }
        }
    });

    wasm_bindgen_futures::spawn_local(async move {
        let should_run = poll_fn(|cx| {
            register_worker_waker(cx.waker());
            control.worker_notify_pending.store(false, Ordering::SeqCst);
            if control.closed.load(Ordering::Acquire) {
                return Poll::Ready(false);
            }
            if control.activated.load(Ordering::Acquire) {
                return Poll::Ready(true);
            }
            Poll::Pending
        })
        .await;
        if !should_run {
            control.stream_done.store(true, Ordering::Release);
            drop(network_tx);
            drop(final_event_tx);
            wake_all_worker_tasks();
            return;
        }

        let close_ctrl = Arc::clone(&control);
        let wait_closed = poll_fn(|cx| {
            register_worker_waker(cx.waker());
            close_ctrl
                .worker_notify_pending
                .store(false, Ordering::SeqCst);
            if close_ctrl.closed.load(Ordering::Acquire) {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        });

        let stream_fut = bbb_client_net::run_event_stream(options, network_tx, worker_cmd_rx);
        let disconnect_reason = tokio::select! {
            biased;
            () = wait_closed => None,
            outcome = stream_fut => match outcome {
                Ok(EventStreamExit::LocalDisconnect) => None,
                Ok(EventStreamExit::RemoteDisconnect(reason)) => Some(reason),
                Err(error) => {
                    Some(crate::network_runtime::format_network_disconnect_reason(&error))
                }
            },
        };

        if let Some(reason) = disconnect_reason {
            let _ = final_event_tx
                .send(NetEvent::Disconnected {
                    reason: Some(reason),
                })
                .await;
        }
        drop(final_event_tx);
        control.stream_done.store(true, Ordering::Release);
        wake_all_worker_tasks();
    });
}
