//! Store screen state and the background job that installs, updates or
//! removes a plugin (spec §5.2 Store).

use iced::Task;
use iced::futures::SinkExt;
use iced::futures::channel::mpsc;
use std::path::Path;

use saveforge_host::{PluginEngine, PluginEntry, PluginRegistry};
use saveforge_store::CatalogEntry;
use saveforge_store::cache::CatalogLoad;
use saveforge_store::install::{install, remove_version};
use semver::Version;

use crate::Message;

#[derive(Debug, Clone, Default)]
pub enum CatalogState {
    #[default]
    NotLoaded,
    Loading,
    Ready(CatalogLoad),
    Failed(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobKind {
    Install,
    Update,
    Remove,
}

/// The one install, update or remove in flight.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Job {
    pub request: u64,
    pub plugin_id: String,
    pub kind: JobKind,
    /// Bytes downloaded and the total, when known.
    pub progress: Option<(u64, Option<u64>)>,
}

#[derive(Debug, Clone)]
pub enum JobOutcome {
    Installed {
        entry: PluginEntry,
        name: String,
        version: Version,
        /// The version an update replaced.
        replaced: Option<Version>,
    },
    Removed {
        id: String,
        name: String,
    },
}

#[derive(Debug, Clone, Default)]
pub struct StoreState {
    pub catalog: CatalogState,
    pub query: String,
    pub job: Option<Job>,
}

/// `done / total`, when the total is known and not zero.
#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)] // a progress bar needs no more than f32
pub fn progress_fraction(progress: Option<(u64, Option<u64>)>) -> Option<f32> {
    match progress? {
        (done, Some(total)) if total > 0 => Some((done as f64 / total as f64) as f32),
        _ => None,
    }
}

type Progress<'a> = &'a mut dyn FnMut(u64, Option<u64>);

/// Installs `entry` and loads it. For an update, `replaced` is the version
/// it supersedes: it is removed only once the new version has loaded, and a
/// new version that fails to load is removed instead, keeping the old one.
pub fn run_install(
    engine: &PluginEngine,
    plugins_dir: &Path,
    entry: &CatalogEntry,
    replaced: Option<Version>,
    progress: Progress<'_>,
) -> Result<JobOutcome, String> {
    install(entry, plugins_dir, progress).map_err(|e| e.to_string())?;
    let loaded = PluginRegistry::load_entry(engine, plugins_dir, &entry.id);
    if let Some(old) = replaced.as_ref().filter(|v| **v != entry.latest_version) {
        if let PluginEntry::Failed { reason, .. } = &loaded {
            if let Err(e) = remove_version(plugins_dir, &entry.id, &entry.latest_version) {
                tracing::warn!(plugin = %entry.id, error = %e, "could not remove the failed version");
            }
            return Err(format!(
                "{} {} could not be loaded ({reason}); keeping {old}.",
                entry.name, entry.latest_version
            ));
        }
        if let Err(e) = remove_version(plugins_dir, &entry.id, old) {
            tracing::warn!(plugin = %entry.id, %old, error = %e, "could not remove the old version");
        }
    }
    Ok(JobOutcome::Installed {
        entry: loaded,
        name: entry.name.clone(),
        version: entry.latest_version.clone(),
        replaced,
    })
}

/// Runs `work` on the blocking pool, streaming its progress as
/// [`Message::JobProgress`] and finishing with [`Message::JobFinished`].
pub(crate) fn job_task(
    request: u64,
    work: impl FnOnce(Progress<'_>) -> Result<JobOutcome, String> + Send + 'static,
) -> Task<Message> {
    let stream = iced::stream::channel(32, move |mut out: mpsc::Sender<Message>| async move {
        let mut progress_out = out.clone();
        let result = tokio::task::spawn_blocking(move || {
            work(&mut |done, total| {
                // A full channel drops this update; a later one follows.
                let _ = progress_out.try_send(Message::JobProgress {
                    request,
                    done,
                    total,
                });
            })
        })
        .await
        .unwrap_or_else(|e| Err(format!("the job failed unexpectedly: {e}")));
        let _ = out.send(Message::JobFinished { request, result }).await;
    });
    Task::stream(stream)
}
