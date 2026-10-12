//! File logging and the crash report (spec §7).

use std::io::Write as _;
use std::path::{Path, PathBuf};

use saveforge_core::paths::AppPaths;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{Builder, Rotation};
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::prelude::*;

/// Days of daily log files kept in `<data_dir>/logs`.
pub const LOG_DAYS_KEPT: usize = 7;

/// Logs to a daily rolling file (and stderr in debug builds). Keep the
/// returned guard alive for the life of the process so buffered lines flush.
pub fn init(paths: &AppPaths) -> Option<WorkerGuard> {
    let level = if cfg!(debug_assertions) {
        LevelFilter::DEBUG
    } else {
        LevelFilter::INFO
    };
    let appender = Builder::new()
        .rotation(Rotation::DAILY)
        .filename_prefix("saveforge")
        .filename_suffix("log")
        .max_log_files(LOG_DAYS_KEPT)
        .build(paths.logs_dir());
    let (file_layer, guard) = match appender {
        Ok(appender) => {
            let (writer, guard) = tracing_appender::non_blocking(appender);
            (
                Some(fmt::layer().with_writer(writer).with_ansi(false)),
                Some(guard),
            )
        }
        Err(e) => {
            eprintln!("logging to file disabled: {e}");
            (None, None)
        }
    };
    let stderr_layer = cfg!(debug_assertions).then(|| fmt::layer().with_writer(std::io::stderr));
    let _ = tracing_subscriber::registry()
        .with(level)
        .with(file_layer)
        .with(stderr_layer)
        .try_init();
    guard
}

/// Writes `crash.log` on panic, then runs the default hook. With
/// `panic = "abort"` in release builds this is the last code that runs.
pub fn install_panic_hook(crash_log: PathBuf) {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let report = format!(
            "{info}\n\nbacktrace:\n{}",
            std::backtrace::Backtrace::force_capture()
        );
        if let Err(e) = write_crash_report(&crash_log, &report) {
            eprintln!("could not write crash report: {e}");
        }
        tracing::error!("panic: {info}");
        previous(info);
    }));
}

/// Replaces `path` with `report`, creating parent directories.
pub fn write_crash_report(path: &Path, report: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = std::fs::File::create(path)?;
    writeln!(file, "SaveForge {} crashed.", env!("CARGO_PKG_VERSION"))?;
    file.write_all(report.as_bytes())
}
