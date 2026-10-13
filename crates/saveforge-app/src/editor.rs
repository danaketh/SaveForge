//! The open document: cached UI tree, host-side UI state, dirty and crash
//! state (spec §5.2 Edit).

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use saveforge_host::{DocumentSession, Event, UiTree};

use crate::documents::{Failure, OpenedDocument, lock};
use crate::render::UiMessage;
use crate::ui_state::{UiState, find_number_input};

#[derive(Debug)]
pub struct Editor {
    pub plugin_id: String,
    pub path: PathBuf,
    session: Arc<Mutex<DocumentSession>>,
    pub tree: UiTree,
    pub ui: UiState,
    pub dirty: bool,
    pub saving: bool,
    /// Request number of the save in flight; other save results are ignored.
    pub save_request: Option<u64>,
    /// Set once the plugin crashed; the document can only be closed.
    pub crashed: Option<String>,
}

/// What the app must do after the editor handled a message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorEffect {
    None,
    /// The plugin crashed or timed out; count it against the plugin.
    PluginFault,
}

impl Editor {
    pub fn new(opened: OpenedDocument) -> Self {
        let mut ui = UiState::default();
        ui.retain(&opened.tree);
        Self {
            plugin_id: opened.plugin_id,
            path: opened.path,
            session: opened.session,
            tree: opened.tree,
            ui,
            dirty: false,
            saving: false,
            save_request: None,
            crashed: None,
        }
    }

    pub fn session(&self) -> Arc<Mutex<DocumentSession>> {
        Arc::clone(&self.session)
    }

    pub fn file_name(&self) -> String {
        self.path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    /// True when closing would lose work (a crashed document can't be saved).
    pub fn has_unsaved_changes(&self) -> bool {
        self.dirty && self.crashed.is_none()
    }

    /// Applies a renderer message, calling the plugin synchronously
    /// (spec §3.4). Ignored while saving or after a crash.
    pub fn handle(&mut self, message: UiMessage) -> EditorEffect {
        if self.saving || self.crashed.is_some() {
            return EditorEffect::None;
        }
        let event = match message {
            UiMessage::Plugin(Event::ListAdd((id, value))) => {
                self.ui.clear_list_choice(&id);
                Event::ListAdd((id, value))
            }
            UiMessage::Plugin(event) => event,
            UiMessage::ListChoice { id, value } => {
                self.ui.set_list_choice(&id, value);
                return EditorEffect::None;
            }
            UiMessage::NumberEdited { id, text } => {
                let Some(input) = find_number_input(&self.tree, &id).copied() else {
                    return EditorEffect::None;
                };
                match self.ui.edit_number(&id, text, &input) {
                    Some(value) => Event::NumberChanged((id, value)),
                    None => return EditorEffect::None,
                }
            }
        };
        self.dispatch(&event)
    }

    fn dispatch(&mut self, event: &Event) -> EditorEffect {
        let result = {
            let mut session = lock(&self.session);
            session
                .update(event)
                .and_then(|()| session.view())
                .and_then(|tree| session.is_dirty().map(|dirty| (tree, dirty)))
        };
        match result {
            Ok((tree, dirty)) => {
                self.ui.retain(&tree);
                self.tree = tree;
                self.dirty = dirty;
                EditorEffect::None
            }
            Err(error) => self.crash(&Failure::from_host(&error)),
        }
    }

    /// Marks the document crashed. Returns whether it counts as a plugin fault.
    pub fn crash(&mut self, failure: &Failure) -> EditorEffect {
        self.crashed = Some(failure.message.clone());
        self.saving = false;
        if failure.plugin_fault {
            EditorEffect::PluginFault
        } else {
            EditorEffect::None
        }
    }

    pub fn saved(&mut self) {
        self.saving = false;
        self.dirty = false;
    }
}
