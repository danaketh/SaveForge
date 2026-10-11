//! Runs `SaveForge` plugin components in a sandbox (spec §3.4, §4).

mod engine;
mod error;
mod stdio;

pub use engine::{HOST_VERSION, HostLimits, LoadedPlugin, PluginEngine, TICK};
pub use error::{HostError, PluginFault};
