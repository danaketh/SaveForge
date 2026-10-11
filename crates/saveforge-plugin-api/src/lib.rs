//! Host-side bindings for the `saveforge:plugin` WIT world (spec §4.4) and
//! UI-tree validation (spec §4.5).

mod tree;

use semver::Version;

#[allow(clippy::all, clippy::pedantic)]
mod bindings {
    wasmtime::component::bindgen!({
        world: "plugin",
        path: "../../wit",
        additional_derives: [PartialEq],
    });
}

pub use bindings::exports::saveforge::plugin::editor::AuxFile;
pub use bindings::saveforge::plugin::host::{Host as LogHost, Level};
pub use bindings::saveforge::plugin::ui::{self, Host as UiHost};
pub use bindings::{Plugin as PluginWorld, PluginPre as PluginWorldPre};
pub use tree::{MAX_TREE_NODES, validate_tree};

/// Version of the WIT package this host implements (`saveforge:plugin@0.1.0`).
pub const API_VERSION: Version = Version::new(0, 1, 0);
