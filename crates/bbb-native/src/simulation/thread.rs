//! Deadline- and producer-woken execution for the single simulation owner.

use std::{
    any::Any,
    panic::{self, AssertUnwindSafe},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc::{self, Receiver, TryRecvError},
        Arc,
    },
    thread::{self, JoinHandle},
    time::Instant,
};

use super::{commands::SimulationMailbox, SimulationCore, SimulationStepResult};
use crate::crash_report::RuntimeFailure;

pub(crate) struct SimulationThread {
    outputs: Receiver<SimulationStepResult>,
    output_backlog: Arc<OutputBacklog>,
    join: Option<JoinHandle<Result<(), RuntimeFailure>>>,
    mailbox: SimulationMailbox,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SimulationOutputBacklog {
    pub(crate) pending: usize,
    pub(crate) peak: usize,
    pub(crate) render_wakes_sent: usize,
    pub(crate) render_wakes_coalesced: usize,
}

#[derive(Default)]
struct OutputBacklog {
    pending: AtomicUsize,
    peak: AtomicUsize,
    render_wake_pending: AtomicBool,
    render_wakes_sent: AtomicUsize,
    render_wakes_coalesced: AtomicUsize,
}

impl SimulationThread {
    pub(crate) fn spawn(
        core: SimulationCore,
        render_wake: impl Fn() + Send + 'static,
    ) -> std::io::Result<Self> {
        let mailbox = core.command_mailbox.clone();
        let scheduler_mailbox = mailbox.clone();
        let (output_tx, outputs) = mpsc::channel();
        let output_backlog = Arc::new(OutputBacklog::default());
        let scheduler_backlog = Arc::clone(&output_backlog);
        let join = thread::Builder::new()
            .name("bbb-simulation".to_string())
            .spawn(move || {
                let _exit_wake = ExitWake(&render_wake);
                let result = panic::catch_unwind(AssertUnwindSafe(|| {
                    let mut core = core;
                    run_scheduled(
                        scheduler_mailbox,
                        output_tx,
                        scheduler_backlog,
                        &render_wake,
                        move |now, tick_deadline_reached| {
                            let output = core.step(now, tick_deadline_reached);
                            let running = output
                                .as_ref()
                                .map_or_else(|| core.running(), SimulationStepResult::running);
                            let deadline = output.as_ref().map(|_| core.next_tick_deadline(now));
                            ScheduledStep {
                                output,
                                running,
                                deadline,
                            }
                        },
                    )
                }));
                match result {
                    Ok(result) => result,
                    Err(payload) => Err(RuntimeFailure::new(
                        "simulation thread",
                        "Simulation thread panicked",
                        panic_payload(payload.as_ref()),
                    )),
                }
            })?;
        Ok(Self {
            outputs,
            output_backlog,
            join: Some(join),
            mailbox,
        })
    }

    pub(crate) fn try_recv(&self) -> Result<SimulationStepResult, TryRecvError> {
        receive_output(&self.outputs, &self.output_backlog)
    }

    pub(crate) fn output_backlog(&self) -> SimulationOutputBacklog {
        self.output_backlog.snapshot()
    }

    /// Joins an already-finished worker without ever blocking the event loop.
    pub(crate) fn poll_finished(&mut self) -> Option<Result<(), RuntimeFailure>> {
        let join = self.join.as_ref()?;
        if !join.is_finished() {
            return None;
        }
        Some(join_result(self.join.take().expect("finished join handle")))
    }

    /// Requests the ordered simulation shutdown and joins before backend state
    /// is dropped by the event-loop owner.
    pub(crate) fn shutdown_and_join(&mut self) -> Result<(), RuntimeFailure> {
        let Some(join) = self.join.take() else {
            return Ok(());
        };
        self.mailbox.enqueue_or_report(
            super::commands::SimulationCommand::Shutdown,
            "simulation thread shutdown",
        );
        join_result(join)
    }
}

impl Drop for SimulationThread {
    fn drop(&mut self) {
        if self.join.is_some() {
            let _ = self.shutdown_and_join();
        }
    }
}

struct ScheduledStep<T> {
    output: Option<T>,
    running: bool,
    deadline: Option<Instant>,
}

struct ExitWake<'a, F: Fn()>(&'a F);

impl<F: Fn()> Drop for ExitWake<'_, F> {
    fn drop(&mut self) {
        (self.0)();
    }
}

struct MailboxCloseGuard(SimulationMailbox);

impl Drop for MailboxCloseGuard {
    fn drop(&mut self) {
        self.0.close();
    }
}

fn run_scheduled<T>(
    mailbox: SimulationMailbox,
    outputs: mpsc::Sender<T>,
    output_backlog: Arc<OutputBacklog>,
    render_wake: impl Fn(),
    mut step: impl FnMut(Instant, bool) -> ScheduledStep<T>,
) -> Result<(), RuntimeFailure> {
    let _close_mailbox_on_exit = MailboxCloseGuard(mailbox.clone());
    let mut deadline = Instant::now();
    loop {
        // Capture before stepping so work published while this step is running
        // cannot be lost between the input barrier and the blocking wait.
        let observed_wake = mailbox.wake_generation();
        let now = Instant::now();
        let scheduled = step(now, now >= deadline);
        let running = scheduled.running;
        if let Some(next_deadline) = scheduled.deadline {
            deadline = next_deadline;
        }
        if let Some(output) = scheduled.output {
            output_backlog.record_enqueue();
            if outputs.send(output).is_err() {
                output_backlog.record_dequeue();
                return Err(RuntimeFailure::new(
                    "simulation publication",
                    "Simulation output receiver closed",
                    "the render owner dropped its simulation output receiver",
                ));
            }
            output_backlog.request_render_wake(&render_wake);
        }
        if !running {
            return Ok(());
        }
        mailbox.wait_for_wake_since(observed_wake, deadline);
    }
}

fn receive_output<T>(
    outputs: &Receiver<T>,
    output_backlog: &OutputBacklog,
) -> Result<T, TryRecvError> {
    match outputs.try_recv() {
        Ok(output) => {
            output_backlog.record_dequeue();
            Ok(output)
        }
        Err(TryRecvError::Empty) => {
            // Clear only after the queue has drained, then recheck to close
            // the producer race. A producer that publishes after the recheck
            // observes false and sends the next event-loop wake.
            output_backlog
                .render_wake_pending
                .store(false, Ordering::Release);
            match outputs.try_recv() {
                Ok(output) => {
                    output_backlog
                        .render_wake_pending
                        .store(true, Ordering::Release);
                    output_backlog.record_dequeue();
                    Ok(output)
                }
                Err(error) => Err(error),
            }
        }
        Err(TryRecvError::Disconnected) => {
            output_backlog
                .render_wake_pending
                .store(false, Ordering::Release);
            Err(TryRecvError::Disconnected)
        }
    }
}

impl OutputBacklog {
    fn record_enqueue(&self) {
        let pending = self.pending.fetch_add(1, Ordering::Relaxed) + 1;
        self.peak.fetch_max(pending, Ordering::Relaxed);
    }

    fn record_dequeue(&self) {
        let previous = self.pending.fetch_sub(1, Ordering::Relaxed);
        debug_assert!(previous > 0, "output backlog underflow");
    }

    fn request_render_wake(&self, render_wake: impl Fn()) {
        if self.render_wake_pending.swap(true, Ordering::AcqRel) {
            self.render_wakes_coalesced.fetch_add(1, Ordering::Relaxed);
        } else {
            self.render_wakes_sent.fetch_add(1, Ordering::Relaxed);
            render_wake();
        }
    }

    fn snapshot(&self) -> SimulationOutputBacklog {
        let pending = self.pending.load(Ordering::Relaxed);
        SimulationOutputBacklog {
            pending,
            // The producer updates pending immediately before peak. Preserve
            // the gauge invariant if a render-thread sample lands between
            // those two relaxed atomic operations.
            peak: self.peak.load(Ordering::Relaxed).max(pending),
            render_wakes_sent: self.render_wakes_sent.load(Ordering::Relaxed),
            render_wakes_coalesced: self.render_wakes_coalesced.load(Ordering::Relaxed),
        }
    }
}

fn join_result(join: JoinHandle<Result<(), RuntimeFailure>>) -> Result<(), RuntimeFailure> {
    join.join().unwrap_or_else(|payload| {
        Err(RuntimeFailure::new(
            "simulation join",
            "Simulation join observed panic",
            panic_payload(payload.as_ref()),
        ))
    })
}

fn panic_payload(payload: &(dyn Any + Send)) -> &str {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("non-string panic payload")
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{
            atomic::{AtomicBool, AtomicUsize, Ordering},
            Arc,
        },
        time::Duration,
    };

    use super::*;
    use crate::simulation::commands::{SimulationCommand, SimulationMailboxSubmitError};

    #[test]
    fn blocked_render_consumer_does_not_stall_deadlines_or_external_network_wake() {
        let mailbox = SimulationMailbox::default();
        let scheduler_mailbox = mailbox.clone();
        let network_pending = Arc::new(AtomicBool::new(false));
        let stepped = Arc::new(AtomicUsize::new(0));
        let network_applied = Arc::new(AtomicUsize::new(0));
        let (output_tx, output_rx) = mpsc::channel::<usize>();
        let output_backlog = Arc::new(OutputBacklog::default());
        let worker_backlog = Arc::clone(&output_backlog);
        let (progress_tx, progress_rx) = mpsc::channel();
        let worker_network = Arc::clone(&network_pending);
        let worker_stepped = Arc::clone(&stepped);
        let worker_applied = Arc::clone(&network_applied);
        let worker_mailbox = mailbox.clone();
        let worker = thread::spawn(move || {
            run_scheduled(
                scheduler_mailbox,
                output_tx,
                worker_backlog,
                || {},
                move |now, _tick_deadline_reached| {
                    let commands = worker_mailbox.drain_through_barrier();
                    let running = !commands
                        .iter()
                        .any(|command| matches!(command.command, SimulationCommand::Shutdown));
                    if worker_network.swap(false, Ordering::SeqCst) {
                        worker_applied.fetch_add(1, Ordering::SeqCst);
                    }
                    let count = worker_stepped.fetch_add(1, Ordering::SeqCst) + 1;
                    let _ = progress_tx.send(count);
                    ScheduledStep {
                        output: Some(count),
                        running,
                        deadline: Some(now + Duration::from_millis(10)),
                    }
                },
            )
        });

        // Keep the fake renderer blocked by deliberately retaining but never
        // draining its unbounded output receiver.
        assert_eq!(
            progress_rx
                .recv_timeout(Duration::from_millis(200))
                .unwrap(),
            1
        );
        network_pending.store(true, Ordering::SeqCst);
        mailbox.notify_external_work();
        while network_applied.load(Ordering::SeqCst) == 0 {
            progress_rx
                .recv_timeout(Duration::from_millis(200))
                .expect("external network wake should run a step");
        }
        while stepped.load(Ordering::SeqCst) < 5 {
            progress_rx
                .recv_timeout(Duration::from_millis(200))
                .expect("tick deadline should continue while render is blocked");
        }
        mailbox.enqueue(SimulationCommand::Shutdown).unwrap();
        assert_eq!(worker.join().unwrap(), Ok(()));
        assert_eq!(
            mailbox.enqueue(SimulationCommand::FocusChanged(true)),
            Err(SimulationMailboxSubmitError::Closed)
        );
        assert!(network_applied.load(Ordering::SeqCst) >= 1);
        assert!(stepped.load(Ordering::SeqCst) >= 5);
        let blocked_backlog = output_backlog.snapshot();
        assert!(blocked_backlog.pending >= 5);
        assert!(blocked_backlog.peak >= blocked_backlog.pending);
        assert_eq!(blocked_backlog.render_wakes_sent, 1);
        assert!(blocked_backlog.render_wakes_coalesced >= 4);
        while receive_output(&output_rx, &output_backlog).is_ok() {}
        let drained_backlog = output_backlog.snapshot();
        assert_eq!(drained_backlog.pending, 0);
        assert!(drained_backlog.peak >= blocked_backlog.peak);
    }

    #[test]
    fn producer_wake_without_work_emits_no_output_and_preserves_deadline() {
        let mailbox = SimulationMailbox::default();
        let scheduler_mailbox = mailbox.clone();
        let worker_mailbox = mailbox.clone();
        let (output_tx, output_rx) = mpsc::channel::<usize>();
        let output_backlog = Arc::new(OutputBacklog::default());
        let worker_backlog = Arc::clone(&output_backlog);
        let (attempt_tx, attempt_rx) = mpsc::channel();
        let attempts = Arc::new(AtomicUsize::new(0));
        let worker_attempts = Arc::clone(&attempts);
        let worker = thread::spawn(move || {
            run_scheduled(
                scheduler_mailbox,
                output_tx,
                worker_backlog,
                || {},
                move |now, tick_deadline_reached| {
                    let attempt = worker_attempts.fetch_add(1, Ordering::SeqCst) + 1;
                    let commands = worker_mailbox.drain_through_barrier();
                    let shutdown = commands
                        .iter()
                        .any(|command| matches!(command.command, SimulationCommand::Shutdown));
                    let output = (attempt == 1 || shutdown).then_some(attempt);
                    let deadline = (attempt == 1).then_some(now + Duration::from_secs(1));
                    let _ = attempt_tx.send((attempt, tick_deadline_reached, output.is_some()));
                    ScheduledStep {
                        output,
                        running: !shutdown,
                        deadline,
                    }
                },
            )
        });

        assert_eq!(
            attempt_rx.recv_timeout(Duration::from_millis(200)).unwrap(),
            (1, true, true)
        );
        mailbox.notify_external_work();
        assert_eq!(
            attempt_rx.recv_timeout(Duration::from_millis(200)).unwrap(),
            (2, false, false)
        );
        mailbox.enqueue(SimulationCommand::Shutdown).unwrap();
        assert_eq!(
            attempt_rx.recv_timeout(Duration::from_millis(200)).unwrap(),
            (3, false, true)
        );
        assert_eq!(worker.join().unwrap(), Ok(()));
        assert_eq!(
            mailbox.enqueue(SimulationCommand::FocusChanged(true)),
            Err(SimulationMailboxSubmitError::Closed)
        );
        assert_eq!(receive_output(&output_rx, &output_backlog), Ok(1));
        assert_eq!(receive_output(&output_rx, &output_backlog), Ok(3));
        assert!(matches!(
            receive_output(&output_rx, &output_backlog),
            Err(TryRecvError::Empty | TryRecvError::Disconnected)
        ));
        assert_eq!(output_backlog.snapshot().pending, 0);
    }

    #[test]
    fn worker_panic_is_reported_as_a_join_failure() {
        let mailbox = SimulationMailbox::default();
        let (output_tx, _outputs) = mpsc::channel::<()>();
        let output_backlog = Arc::new(OutputBacklog::default());
        let join = thread::spawn(move || {
            panic::catch_unwind(AssertUnwindSafe(|| {
                run_scheduled(
                    mailbox,
                    output_tx,
                    output_backlog,
                    || {},
                    |_now, _tick_deadline_reached| -> ScheduledStep<()> {
                        panic!("fake simulation failure")
                    },
                )
            }))
            .map_err(|payload| {
                RuntimeFailure::new(
                    "simulation thread",
                    "Simulation thread panicked",
                    panic_payload(payload.as_ref()),
                )
            })
            .and_then(|result| result)
        });

        let error = join_result(join).unwrap_err();
        assert!(error.to_string().contains("fake simulation failure"));
    }
}
