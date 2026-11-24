use std::io;

/// Which signal asked the process to stop.
///
/// A supervisor sends one of these and then reads the log to find out whether
/// the server saved before it went; naming the signal is what makes that record
/// answer the question rather than merely note that something happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownSignal {
    /// `SIGINT`, which is what a terminal sends on Ctrl-C.
    Interrupt,
    /// `SIGTERM`, which is what systemd and launchd send to stop a service.
    Terminate,
}

impl ShutdownSignal {
    pub fn name(self) -> &'static str {
        match self {
            Self::Interrupt => "SIGINT",
            Self::Terminate => "SIGTERM",
        }
    }
}

/// The process-owned graceful shutdown signals, with their handlers already
/// installed.
///
/// Installing is separate from waiting because the two answer different
/// questions. A supervisor that has been told the server is up may stop it at
/// once, and until a handler is installed the default disposition for `SIGTERM`
/// is to kill the process outright -- no save, no log line, nothing to read
/// afterwards. So the handlers go in before readiness is announced, and only
/// then does anything wait on them.
///
/// Vanilla arms its own stop path differently and later: the JVM shutdown hook
/// that calls `halt(true)` is added by `Main` after `spin` has returned the
/// running server (`Main.java:213-220`), while the readiness line is printed on
/// the server thread as soon as `loadLevel` returns (`DedicatedServer.java:282`,
/// `:285`). This is therefore bbb's own supervisor contract rather than a
/// transcription -- what is transcribed is where readiness is announced.
#[cfg(unix)]
pub struct ShutdownSignals {
    interrupt: tokio::signal::unix::Signal,
    terminate: tokio::signal::unix::Signal,
}

#[cfg(unix)]
impl ShutdownSignals {
    /// Install both handlers now. `tokio::signal::unix::signal` registers with
    /// the OS as it is called, so a signal that arrives after this returns is
    /// delivered to the stream rather than to the default disposition.
    pub fn install() -> io::Result<Self> {
        use tokio::signal::unix::{signal, SignalKind};

        Ok(Self {
            interrupt: signal(SignalKind::interrupt())?,
            terminate: signal(SignalKind::terminate())?,
        })
    }

    /// Wait for whichever of the two arrives first.
    pub async fn recv(&mut self) -> ShutdownSignal {
        tokio::select! {
            _ = self.interrupt.recv() => ShutdownSignal::Interrupt,
            _ = self.terminate.recv() => ShutdownSignal::Terminate,
        }
    }
}

#[cfg(not(unix))]
pub struct ShutdownSignals {
    _private: (),
}

#[cfg(not(unix))]
impl ShutdownSignals {
    pub fn install() -> io::Result<Self> {
        Ok(Self { _private: () })
    }

    pub async fn recv(&mut self) -> ShutdownSignal {
        let _ = tokio::signal::ctrl_c().await;
        ShutdownSignal::Interrupt
    }
}
