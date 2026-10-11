//! Runs `SaveForge` plugin components in a sandbox (spec §3.4, §4).

mod aux_files;
mod detect;
mod engine;
mod error;
mod registry;
mod session;
mod stdio;

pub use aux_files::resolve_aux_files;
pub use detect::{DetectReport, Detection, HEADER_LEN, detect};
pub use engine::{HOST_VERSION, HostLimits, LoadedPlugin, PluginEngine, TICK};
pub use error::{HostError, PluginFault};
pub use registry::{MAX_FAULTS_PER_SESSION, PluginEntry, PluginRegistry};
pub use saveforge_plugin_api::AuxFile;
pub use saveforge_plugin_api::ui::{Event, UiTree};
pub use session::DocumentSession;
