//! A live document: one store + instance + `document` resource (spec §3.4).

use std::fmt;

use saveforge_plugin_api::ui::{Event, UiTree};
use saveforge_plugin_api::{AuxFile, PluginWorld, validate_tree};
use wasmtime::Store;
use wasmtime::component::ResourceAny;

use crate::engine::{LoadedPlugin, PluginEngine, State};
use crate::error::{HostError, PluginFault, classify};

impl PluginEngine {
    /// Asks `plugin` whether it can open the file. Uses a throwaway instance.
    pub fn detect(
        &self,
        plugin: &LoadedPlugin,
        file_name: &str,
        header: &[u8],
    ) -> Result<Option<u32>, HostError> {
        let (mut store, world) = self.instantiate(plugin)?;
        store.set_epoch_deadline(self.io_ticks());
        world
            .saveforge_plugin_editor()
            .call_detect(&mut store, file_name, header)
            .map_err(|e| HostError::Fault(classify(&e)))
    }

    /// Opens a save in a fresh instance owned by the returned session.
    pub fn open(
        &self,
        plugin: &LoadedPlugin,
        file_name: &str,
        bytes: &[u8],
        aux: &[AuxFile],
    ) -> Result<DocumentSession, HostError> {
        let (mut store, world) = self.instantiate(plugin)?;
        store.set_epoch_deadline(self.io_ticks());
        let doc = world
            .saveforge_plugin_editor()
            .call_open(&mut store, file_name, bytes, aux)
            .map_err(|e| HostError::Fault(classify(&e)))?
            .map_err(HostError::Rejected)?;
        Ok(DocumentSession {
            plugin_id: plugin.manifest.id.clone(),
            store,
            world,
            doc,
            ui_ticks: self.ui_ticks(),
            io_ticks: self.io_ticks(),
            poisoned: false,
        })
    }
}

/// An open document. After any [`PluginFault`] the session is poisoned and
/// every later call returns [`HostError::Poisoned`].
pub struct DocumentSession {
    plugin_id: String,
    store: Store<State>,
    world: PluginWorld,
    doc: ResourceAny,
    ui_ticks: u64,
    io_ticks: u64,
    poisoned: bool,
}

impl fmt::Debug for DocumentSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DocumentSession")
            .field("plugin_id", &self.plugin_id)
            .field("poisoned", &self.poisoned)
            .finish_non_exhaustive()
    }
}

impl DocumentSession {
    pub fn plugin_id(&self) -> &str {
        &self.plugin_id
    }

    pub fn is_poisoned(&self) -> bool {
        self.poisoned
    }

    pub fn view(&mut self) -> Result<UiTree, HostError> {
        let tree = self.call(self.ui_ticks, |w, s, d| {
            w.saveforge_plugin_editor().document().call_view(s, d)
        })?;
        if let Err(reason) = validate_tree(&tree) {
            self.poisoned = true;
            return Err(HostError::Fault(PluginFault::InvalidUi(reason)));
        }
        Ok(tree)
    }

    pub fn update(&mut self, event: &Event) -> Result<(), HostError> {
        self.call(self.ui_ticks, |w, s, d| {
            w.saveforge_plugin_editor()
                .document()
                .call_update(s, d, event)
        })
    }

    pub fn is_dirty(&mut self) -> Result<bool, HostError> {
        self.call(self.ui_ticks, |w, s, d| {
            w.saveforge_plugin_editor().document().call_is_dirty(s, d)
        })
    }

    /// The plugin's bytes for the current state. A plugin-reported error is
    /// [`HostError::SerializeFailed`] and leaves the session usable.
    pub fn serialize(&mut self) -> Result<Vec<u8>, HostError> {
        self.call(self.io_ticks, |w, s, d| {
            w.saveforge_plugin_editor().document().call_serialize(s, d)
        })?
        .map_err(HostError::SerializeFailed)
    }

    pub fn mark_saved(&mut self) -> Result<(), HostError> {
        self.call(self.ui_ticks, |w, s, d| {
            w.saveforge_plugin_editor().document().call_mark_saved(s, d)
        })
    }

    fn call<R>(
        &mut self,
        ticks: u64,
        f: impl FnOnce(&PluginWorld, &mut Store<State>, ResourceAny) -> wasmtime::Result<R>,
    ) -> Result<R, HostError> {
        if self.poisoned {
            return Err(HostError::Poisoned);
        }
        self.store.set_epoch_deadline(ticks);
        f(&self.world, &mut self.store, self.doc).map_err(|e| {
            self.poisoned = true;
            HostError::Fault(classify(&e))
        })
    }
}

impl Drop for DocumentSession {
    fn drop(&mut self) {
        // A poisoned instance may be mid-trap; never re-enter it.
        if self.poisoned {
            return;
        }
        self.store.set_epoch_deadline(self.ui_ticks);
        if let Err(e) = self.doc.resource_drop(&mut self.store) {
            tracing::debug!(plugin = %self.plugin_id, error = %e, "document drop failed");
        }
    }
}
