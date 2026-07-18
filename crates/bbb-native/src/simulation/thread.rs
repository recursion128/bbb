//! Deadline- and producer-woken execution for the single simulation owner.

use std::{
    any::Any,
    panic::{self, AssertUnwindSafe},
    sync::mpsc::{self, Receiver, TryRecvError},
    thread::{self, JoinHandle},
    time::Instant,
};

use super::{commands::SimulationMailbox, SimulationCore, SimulationStepResult};

pub(crate) struct SimulationThread {
    outputs: Receiver<SimulationStepResult>,
    join: Option<JoinHandle<Result<(), String>>>,
    mailbox: SimulationMailbox,
}

impl SimulationThread {
    pub(crate) fn spawn(
        core: SimulationCore,
        render_wake: impl Fn() + Send + 'static,
    ) -> std::io::Result<Self> {
        let mailbox = core.command_mailbox.clone();
        let scheduler_mailbox = mailbox.clone();
        let (output_tx, outputs) = mpsc::channel();
        let join = thread::Builder::new()
            .name("bbb-simulation".to_string())
            .spawn(move || {
                let _exit_wake = ExitWake(&render_wake);
                let result = panic::catch_unwind(AssertUnwindSafe(|| {
                    let mut core = core;
                    run_scheduled(scheduler_mailbox, output_tx, &render_wake, move |now| {
                        let output = core.step(now);
                        let running = output.running();
                        let deadline = core.next_tick_deadline(now);
                        ScheduledStep {
                            output,
                            running,
                            deadline,
                        }
                    })
                }));
                match result {
                    Ok(result) => result,
                    Err(payload) => Err(format!(
                        "simulation thread panicked: {}",
                        panic_payload(payload.as_ref())
                    )),
                }
            })?;
        Ok(Self {
            outputs,
            join: Some(join),
            mailbox,
        })
    }

    pub(crate) fn try_recv(&self) -> Result<SimulationStepResult, TryRecvError> {
        self.outputs.try_recv()
    }

    /// Joins an already-finished worker without ever blocking the event loop.
    pub(crate) fn poll_finished(&mut self) -> Option<Result<(), String>> {
        let join = self.join.as_ref()?;
        if !join.is_finished() {
            return None;
        }
        Some(join_result(self.join.take().expect("finished join handle")))
    }

    /// Requests the ordered simulation shutdown and joins before backend state
    /// is dropped by the event-loop owner.
    pub(crate) fn shutdown_and_join(&mut self) -> Result<(), String> {
        let Some(join) = self.join.take() else {
            return Ok(());
        };
        self.mailbox
            .enqueue(super::commands::SimulationCommand::Shutdown);
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
    output: T,
    running: bool,
    deadline: Instant,
}

struct ExitWake<'a, F: Fn()>(&'a F);

impl<F: Fn()> Drop for ExitWake<'_, F> {
    fn drop(&mut self) {
        (self.0)();
    }
}

fn run_scheduled<T>(
    mailbox: SimulationMailbox,
    outputs: mpsc::Sender<T>,
    render_wake: impl Fn(),
    mut step: impl FnMut(Instant) -> ScheduledStep<T>,
) -> Result<(), String> {
    loop {
        // Capture before stepping so work published while this step is running
        // cannot be lost between the input barrier and the blocking wait.
        let observed_wake = mailbox.wake_generation();
        let scheduled = step(Instant::now());
        let running = scheduled.running;
        let deadline = scheduled.deadline;
        outputs
            .send(scheduled.output)
            .map_err(|_| "simulation output receiver closed".to_string())?;
        render_wake();
        if !running {
            return Ok(());
        }
        mailbox.wait_for_wake_since(observed_wake, deadline);
    }
}

fn join_result(join: JoinHandle<Result<(), String>>) -> Result<(), String> {
    join.join().unwrap_or_else(|payload| {
        Err(format!(
            "simulation join observed panic: {}",
            panic_payload(payload.as_ref())
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
    use crate::simulation::commands::SimulationCommand;

    #[test]
    fn blocked_render_consumer_does_not_stall_deadlines_or_external_network_wake() {
        let mailbox = SimulationMailbox::default();
        let scheduler_mailbox = mailbox.clone();
        let network_pending = Arc::new(AtomicBool::new(false));
        let stepped = Arc::new(AtomicUsize::new(0));
        let network_applied = Arc::new(AtomicUsize::new(0));
        let (output_tx, output_rx) = mpsc::channel::<usize>();
        let (progress_tx, progress_rx) = mpsc::channel();
        let worker_network = Arc::clone(&network_pending);
        let worker_stepped = Arc::clone(&stepped);
        let worker_applied = Arc::clone(&network_applied);
        let worker_mailbox = mailbox.clone();
        let worker = thread::spawn(move || {
            run_scheduled(
                scheduler_mailbox,
                output_tx,
                || {},
                move |now| {
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
                        output: count,
                        running,
                        deadline: now + Duration::from_millis(10),
                    }
                },
            )
        });

        // Keep the fake renderer blocked by deliberately retaining but never
        // draining its unbounded output receiver.
        let _blocked_renderer = output_rx;
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

        mailbox.enqueue(SimulationCommand::Shutdown);
        assert_eq!(worker.join().unwrap(), Ok(()));
        assert!(network_applied.load(Ordering::SeqCst) >= 1);
        assert!(stepped.load(Ordering::SeqCst) >= 5);
    }

    #[test]
    fn worker_panic_is_reported_as_a_join_failure() {
        let mailbox = SimulationMailbox::default();
        let (output_tx, _outputs) = mpsc::channel::<()>();
        let join = thread::spawn(move || {
            panic::catch_unwind(AssertUnwindSafe(|| {
                run_scheduled(
                    mailbox,
                    output_tx,
                    || {},
                    |_now| -> ScheduledStep<()> { panic!("fake simulation failure") },
                )
            }))
            .map_err(|payload| panic_payload(payload.as_ref()).to_string())
            .and_then(|result| result)
        });

        let error = join_result(join).unwrap_err();
        assert!(error.contains("fake simulation failure"));
    }
}
