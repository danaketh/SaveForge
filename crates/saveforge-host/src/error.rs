use std::path::PathBuf;

use saveforge_core::CoreError;
use wasmtime::Trap;

/// Why a plugin call went wrong in a way the plugin did not choose.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PluginFault {
    #[error("timed out")]
    Timeout,
    #[error("{0}")]
    Trap(String),
    #[error("returned an invalid UI: {0}")]
    InvalidUi(String),
}

#[derive(Debug, thiserror::Error)]
pub enum HostError {
    #[error(transparent)]
    Core(#[from] CoreError),
    #[error("could not compile plugin component {}: {reason}", path.display())]
    Compile { path: PathBuf, reason: String },
    #[error("could not instantiate plugin: {0}")]
    Instantiate(String),
    #[error("plugin crashed: {0}")]
    Fault(PluginFault),
    #[error("plugin is no longer usable after a crash")]
    Poisoned,
    #[error("plugin could not open the file: {0}")]
    Rejected(String),
    #[error("plugin could not save the document: {0}")]
    SerializeFailed(String),
    #[error("required file '{0}' was not found")]
    MissingAux(String),
    #[error("could not start the plugin engine: {0}")]
    Engine(String),
}

/// Maps a failed guest call to a fault: epoch interrupts are timeouts,
/// everything else is a trap.
pub(crate) fn classify(error: &wasmtime::Error) -> PluginFault {
    match error.downcast_ref::<Trap>() {
        Some(Trap::Interrupt) => PluginFault::Timeout,
        Some(trap) => PluginFault::Trap(trap.to_string()),
        None => PluginFault::Trap(error.to_string()),
    }
}
