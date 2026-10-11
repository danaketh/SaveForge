//! Wasmtime engine, per-store state and plugin loading (spec §3.4).

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};
use std::thread;
use std::time::Duration;

use saveforge_core::manifest::PluginManifest;
use saveforge_plugin_api::{API_VERSION, Level, LogHost, PluginWorld, PluginWorldPre, UiHost};
use semver::Version;
use wasmtime::component::{Component, HasSelf, Linker, Resource, ResourceTable};
use wasmtime::{Cache, CacheConfig, Config, Engine, Store, StoreLimits, StoreLimitsBuilder};
use wasmtime_wasi::p2::DynPollable;
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

use crate::error::HostError;
use crate::stdio::LogStream;

/// This host's version, checked against `minHostVersion`/`maxHostVersion`.
pub static HOST_VERSION: LazyLock<Version> = LazyLock::new(|| {
    Version::parse(env!("CARGO_PKG_VERSION")).expect("crate version is valid semver")
});

/// Epoch tick length. Call deadlines are counted in ticks.
pub const TICK: Duration = Duration::from_millis(10);

/// Per-call deadlines and per-instance memory cap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostLimits {
    /// `view`, `update`, `is-dirty`, `mark-saved`, resource drop.
    pub ui_call: Duration,
    /// instantiate, `detect`, `open`, `serialize`.
    pub io_call: Duration,
    pub memory_bytes: usize,
}

impl Default for HostLimits {
    fn default() -> Self {
        Self {
            ui_call: Duration::from_millis(250),
            io_call: Duration::from_secs(10),
            memory_bytes: 512 * 1024 * 1024,
        }
    }
}

/// Per-`Store` data: an empty WASI context plus limits and the plugin id for logs.
pub(crate) struct State {
    wasi: WasiCtx,
    table: ResourceTable,
    limits: StoreLimits,
    plugin_id: Arc<str>,
}

impl WasiView for State {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

impl UiHost for State {}

impl LogHost for State {
    fn log(&mut self, level: Level, msg: String) {
        let plugin = &*self.plugin_id;
        match level {
            Level::Trace => tracing::trace!(plugin, "{msg}"),
            Level::Debug => tracing::debug!(plugin, "{msg}"),
            Level::Info => tracing::info!(plugin, "{msg}"),
            Level::Warn => tracing::warn!(plugin, "{msg}"),
            Level::Error => tracing::error!(plugin, "{msg}"),
        }
    }
}

/// A compiled, type-checked plugin ready to instantiate.
pub struct LoadedPlugin {
    pub manifest: PluginManifest,
    pub dir: PathBuf,
    pre: PluginWorldPre<State>,
}

impl fmt::Debug for LoadedPlugin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LoadedPlugin")
            .field("id", &self.manifest.id)
            .field("version", &self.manifest.version)
            .field("dir", &self.dir)
            .finish_non_exhaustive()
    }
}

/// One wasmtime engine for the app, with an epoch ticker thread that runs for
/// as long as anything (this engine, a `LoadedPlugin`, a `DocumentSession`)
/// still holds the underlying wasmtime engine.
pub struct PluginEngine {
    engine: Engine,
    linker: Linker<State>,
    limits: HostLimits,
}

impl fmt::Debug for PluginEngine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PluginEngine")
            .field("limits", &self.limits)
            .finish_non_exhaustive()
    }
}

impl PluginEngine {
    /// `cache_dir` (absolute) enables wasmtime's on-disk compile cache.
    pub fn new(limits: HostLimits, cache_dir: Option<&Path>) -> Result<Self, HostError> {
        let mut config = Config::new();
        config.epoch_interruption(true);
        if let Some(dir) = cache_dir {
            let mut cache = CacheConfig::new();
            cache.with_directory(dir);
            config.cache(Some(Cache::new(cache).map_err(|e| engine_error(&e))?));
        }
        let engine = Engine::new(&config).map_err(|e| engine_error(&e))?;
        let mut linker = Linker::new(&engine);
        wasmtime_wasi::p2::add_to_linker_sync(&mut linker).map_err(|e| engine_error(&e))?;
        make_waiting_non_blocking(&mut linker).map_err(|e| engine_error(&e))?;
        PluginWorld::add_to_linker::<State, HasSelf<State>>(&mut linker, |state| state)
            .map_err(|e| engine_error(&e))?;

        spawn_ticker(&engine);
        Ok(Self {
            engine,
            linker,
            limits,
        })
    }

    pub fn limits(&self) -> HostLimits {
        self.limits
    }

    /// Reads and validates the manifest, checks compatibility, compiles the
    /// component and type-checks it against the `plugin` world.
    pub fn load(&self, dir: &Path) -> Result<LoadedPlugin, HostError> {
        let manifest = PluginManifest::load(dir)?;
        manifest.check_compat(&HOST_VERSION, &API_VERSION)?;
        let path = dir.join(&manifest.component);
        let component =
            Component::from_file(&self.engine, &path).map_err(|e| HostError::Compile {
                path: path.clone(),
                reason: e.to_string(),
            })?;
        let pre = self
            .linker
            .instantiate_pre(&component)
            .and_then(PluginWorldPre::new)
            .map_err(|e| HostError::Compile {
                path,
                reason: e.to_string(),
            })?;
        Ok(LoadedPlugin {
            manifest,
            dir: dir.to_path_buf(),
            pre,
        })
    }

    pub(crate) fn new_store(&self, plugin_id: &str) -> Store<State> {
        let plugin_id: Arc<str> = Arc::from(plugin_id);
        let wasi = WasiCtx::builder()
            .stdout(LogStream::new(Arc::clone(&plugin_id), "stdout"))
            .stderr(LogStream::new(Arc::clone(&plugin_id), "stderr"))
            .build();
        let mut store = Store::new(
            &self.engine,
            State {
                wasi,
                table: ResourceTable::new(),
                limits: StoreLimitsBuilder::new()
                    .memory_size(self.limits.memory_bytes)
                    .build(),
                plugin_id,
            },
        );
        store.limiter(|state| &mut state.limits);
        store
    }

    /// Fresh store + instance for `plugin`, within the io-call deadline.
    pub(crate) fn instantiate(
        &self,
        plugin: &LoadedPlugin,
    ) -> Result<(Store<State>, PluginWorld), HostError> {
        let mut store = self.new_store(&plugin.manifest.id);
        store.set_epoch_deadline(self.io_ticks());
        let world = plugin
            .pre
            .instantiate(&mut store)
            .map_err(|e| HostError::Instantiate(e.to_string()))?;
        Ok((store, world))
    }

    pub(crate) fn ui_ticks(&self) -> u64 {
        ticks(self.limits.ui_call)
    }

    pub(crate) fn io_ticks(&self) -> u64 {
        ticks(self.limits.io_call)
    }
}

/// Epoch interruption only fires while wasm code runs, not while a guest waits
/// inside a WASI host call (e.g. `std::thread::sleep` becomes a monotonic-clock
/// pollable plus `poll`/`block`). With nothing for a plugin to wait on (no
/// files, sockets or stdin), every wait is replaced by "ready now": sleeps
/// return at once and busy-waiting guests stay subject to the deadline.
fn make_waiting_non_blocking(linker: &mut Linker<State>) -> wasmtime::Result<()> {
    linker.allow_shadowing(true);
    let mut poll = linker.instance("wasi:io/poll@0.2.12")?;
    poll.func_wrap(
        "poll",
        |_store, (pollables,): (Vec<Resource<DynPollable>>,)| {
            let count = u32::try_from(pollables.len()).unwrap_or(u32::MAX);
            Ok(((0..count).collect::<Vec<u32>>(),))
        },
    )?;
    poll.func_wrap(
        "[method]pollable.block",
        |_store, (_pollable,): (Resource<DynPollable>,)| Ok(()),
    )?;
    linker.allow_shadowing(false);
    Ok(())
}

fn spawn_ticker(engine: &Engine) {
    // A weak handle: stores and pre-instantiated plugins keep the engine
    // alive after `PluginEngine` is dropped, and their deadlines must still fire.
    let engine = engine.weak();
    thread::Builder::new()
        .name("saveforge-epoch".to_owned())
        .spawn(move || {
            while let Some(engine) = engine.upgrade() {
                engine.increment_epoch();
                drop(engine);
                thread::sleep(TICK);
            }
        })
        .expect("spawn epoch ticker thread");
}

fn ticks(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis().div_ceil(TICK.as_millis()))
        .unwrap_or(u64::MAX)
        .max(1)
}

fn engine_error(error: &wasmtime::Error) -> HostError {
    HostError::Engine(error.to_string())
}
