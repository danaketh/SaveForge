//! Store screen state and the background job that installs, updates or
//! removes a plugin (spec §5.2 Store).

use iced::Task;
use iced::futures::SinkExt;
use iced::futures::channel::mpsc;
use saveforge_host::PluginEntry;
use saveforge_store::cache::CatalogLoad;
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
