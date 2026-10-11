//! Runs `SaveForge` plugin components in a sandbox (spec §3.4, §4).

mod engine;
mod error;
mod session;
mod stdio;

pub use engine::{HOST_VERSION, HostLimits, LoadedPlugin, PluginEngine, TICK};
pub use error::{HostError, PluginFault};
pub use saveforge_plugin_api::AuxFile;
pub use saveforge_plugin_api::ui::{Event, UiTree};
pub use session::DocumentSession;
