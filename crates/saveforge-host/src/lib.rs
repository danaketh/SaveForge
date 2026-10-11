//! Runs `SaveForge` plugin components in a sandbox (spec §3.4, §4).

mod aux_files;
mod engine;
mod error;
mod session;
mod stdio;

pub use aux_files::resolve_aux_files;
pub use engine::{HOST_VERSION, HostLimits, LoadedPlugin, PluginEngine, TICK};
pub use error::{HostError, PluginFault};
pub use saveforge_plugin_api::AuxFile;
pub use saveforge_plugin_api::ui::{Event, UiTree};
pub use session::DocumentSession;
