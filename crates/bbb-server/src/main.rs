use std::process::ExitCode;

use bbb_server::{
    install_panic_report, ServerOptions, ServerRuntime, ShutdownSignals, SERVER_THREAD_STACK_BYTES,
};
use clap::Parser;
use tracing_subscriber::EnvFilter;

/// What the process exits with when it could not do the job it was started for.
///
/// A supervisor restarts on a non-zero status, so the two failures below have
/// to be distinguishable from the clean stop a signal produces.
const EXIT_FAILED: u8 = 1;

fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .json()
        .with_writer(std::io::stderr)
        .init();
    install_panic_report();

    let handle = match std::thread::Builder::new()
        .name("bbb-server-main".into())
        .stack_size(SERVER_THREAD_STACK_BYTES)
        .spawn(|| {
            let result = tokio::runtime::Builder::new_multi_thread()
                .thread_stack_size(SERVER_THREAD_STACK_BYTES)
                .enable_all()
                .build()
                .map_err(|err| -> Box<dyn std::error::Error> { Box::new(err) })
                .and_then(|runtime| runtime.block_on(run()));
            match result {
                Ok(()) => ExitCode::SUCCESS,
                // The error goes to the structured log rather than to `main`'s own
                // `Err` rendering: a supervisor reads one stream, and a failure that
                // only exists as a bare stderr line is not attributable to anything.
                Err(error) => {
                    tracing::error!(event = "server.failed", error = %error);
                    ExitCode::from(EXIT_FAILED)
                }
            }
        }) {
        Ok(handle) => handle,
        Err(error) => {
            tracing::error!(event = "server.failed", error = %error);
            return ExitCode::from(EXIT_FAILED);
        }
    };

    match handle.join() {
        Ok(code) => code,
        Err(_) => {
            tracing::error!(
                event = "server.failed",
                error = "server main thread panicked"
            );
            ExitCode::from(EXIT_FAILED)
        }
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let options = ServerOptions::parse();
    // Chunk generation batches run on the global pool; starting it here keeps
    // the thread count flat after startup.
    rayon::ThreadPoolBuilder::new()
        .stack_size(SERVER_THREAD_STACK_BYTES)
        .thread_name(|index| format!("bbb-worldgen-{index}"))
        .build_global()?;
    // Both handlers are installed before anything else so that no window
    // exists in which the process is alive and a stop would kill it outright.
    // Nothing is waiting on them yet; `run_until` does that, and it announces
    // readiness only once the world behind it is open.
    let mut signals = ShutdownSignals::install()?;
    let runtime = ServerRuntime::bind(options).await?;
    let report = runtime
        .run_until(async move {
            // The signal is named because this line is the one a supervisor
            // reads to tell a stop it asked for from a stop it did not.
            let signal = signals.recv().await;
            tracing::info!(event = "server.signalled", signal = signal.name());
        })
        .await?;
    tracing::info!(
        event = "server.stopped",
        accepted_connections = report.accepted_connections,
        rejected_connections = report.rejected_connections,
        completed_connections = report.completed_connections,
    );
    Ok(())
}
