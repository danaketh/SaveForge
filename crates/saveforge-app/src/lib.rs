//! The `SaveForge` desktop app (spec §5).

mod app;
pub mod documents;
pub mod editor;
pub mod format;
pub mod home;
pub mod logging;
pub mod render;
pub mod theme;
pub mod ui_state;

pub use app::{App, Message, Modal, Pending, Plugins, REPOSITORY_URL, Screen, Services, run};
