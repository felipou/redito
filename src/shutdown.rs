use tokio::signal::unix::{Signal, SignalKind, signal};

/// SIGTERM/SIGINT listener for long-running commands (graceful shutdown).
///
/// The handlers are installed by `install`; a signal that arrives while nobody is
/// waiting in `recv` is kept and returned by the next call.
pub(crate) struct Shutdown {
    sigterm: Signal,
    sigint: Signal,
}

impl Shutdown {
    pub(crate) fn install() -> std::io::Result<Self> {
        Ok(Self {
            sigterm: signal(SignalKind::terminate())?,
            sigint: signal(SignalKind::interrupt())?,
        })
    }

    /// Resolves with the signal's name once one arrives.
    pub(crate) async fn recv(&mut self) -> &'static str {
        tokio::select! {
            _ = self.sigterm.recv() => "SIGTERM",
            _ = self.sigint.recv() => "SIGINT",
        }
    }
}
