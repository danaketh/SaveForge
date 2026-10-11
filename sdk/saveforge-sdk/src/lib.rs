//! Guest-side SDK for `SaveForge` plugins (spec §4.6).
//!
//! Implement [`Plugin`] and [`Document`], then call [`export_plugin!`] once in
//! the plugin crate. Build with `cargo xtask plugins`.

#[doc(hidden)]
#[allow(clippy::all, clippy::pedantic)]
pub mod bindings {
    wit_bindgen::generate!({
        world: "plugin",
        path: "../../wit",
        pub_export_macro: true,
        default_bindings_module: "saveforge_sdk::bindings",
        additional_derives: [PartialEq],
    });
}

mod adapter;
mod log;
pub mod ui;

#[doc(hidden)]
pub use adapter::{Adapter, DocAdapter};
pub use bindings::exports::saveforge::plugin::editor::AuxFile;
pub use log::{Level, log};

/// Entry point of a plugin. Implemented on a unit struct; never instantiated.
pub trait Plugin: 'static {
    type Doc: Document;

    /// `Some(priority)` when this plugin can open the file. `header` holds at
    /// most the first 512 bytes.
    fn detect(file_name: &str, header: &[u8]) -> Option<u32>;

    /// Parses the save. `aux` holds the manifest's aux files that were found.
    fn open(file_name: &str, bytes: Vec<u8>, aux: Vec<AuxFile>) -> Result<Self::Doc, String>;
}

/// An open save file. The host calls `view` after every `update`.
pub trait Document: 'static {
    fn view(&self) -> ui::UiTree;
    fn update(&mut self, event: ui::Event);
    fn is_dirty(&self) -> bool;
    /// Must not change the dirty state; the host calls `mark_saved` after the
    /// bytes are on disk.
    fn serialize(&self) -> Result<Vec<u8>, String>;
    fn mark_saved(&mut self);
}

/// Exports `$plugin` (a type implementing [`Plugin`]) as the component's
/// `editor` interface. Use once per plugin crate, at module level.
#[macro_export]
macro_rules! export_plugin {
    ($plugin:ty) => {
        type __SaveforgeExport = $crate::Adapter<$plugin>;
        $crate::bindings::export!(__SaveforgeExport with_types_in $crate::bindings);
    };
}
