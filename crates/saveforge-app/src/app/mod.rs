//! Application state and update logic (spec §5.1, §5.2). Rendering lives in
//! `view.rs`.

mod settings_view;
mod view;

use std::path::PathBuf;
use std::sync::Arc;

use iced::{Subscription, Task, keyboard, window};
use saveforge_core::discovery::{SaveFileInfo, discover};
use saveforge_core::paths::AppPaths;
use saveforge_core::platform::Os;
use saveforge_core::settings::AppSettings;
use saveforge_core::tokens::TokenEnv;
use saveforge_host::{LoadedPlugin, PluginEngine, PluginEntry, PluginRegistry};
use time::UtcOffset;

use crate::documents::{
    DetectOutcome, Detected, Failure, OpenedDocument, detect_file, open_document, save_document,
};
use crate::editor::{Editor, EditorEffect};
use crate::format::window_title;
use crate::home::{Home, Saves, raw_save_dir, resolve_save_dir};
use crate::render::UiMessage;

/// Source repository, linked from About. Placeholder until the project is
/// published, like `RELEASE_CATALOG_URL`.
pub const REPOSITORY_URL: &str = "https://github.com/YOUR_ORG/YOUR_REPO";

/// The page shown in the main area (spec §5.1). An open document lives on
/// `Home` and survives visits to the other screens.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Screen {
    #[default]
    Home,
    Store,
    PluginDetail(String),
    Settings,
    About,
}

/// Long-lived handles shared with worker threads.
#[derive(Debug, Clone)]
pub struct Services {
    pub engine: Arc<PluginEngine>,
    pub paths: AppPaths,
    pub tokens: Arc<TokenEnv>,
    pub local_offset: UtcOffset,
}

#[derive(Debug, Clone)]
pub enum Plugins {
    Loading,
    Ready(PluginRegistry),
}

/// What to do once the user has dealt with unsaved changes.
#[derive(Debug, Clone, PartialEq)]
pub enum Pending {
    OpenFile(PathBuf),
    SelectPlugin(String),
    CloseEditor,
    Exit(window::Id),
}

#[derive(Debug, Clone)]
pub enum Modal {
    ChoosePlugin {
        path: PathBuf,
        candidates: Vec<Arc<LoadedPlugin>>,
    },
    Unsaved {
        then: Pending,
    },
}

#[derive(Debug, Clone)]
pub enum Message {
    PluginsLoaded(PluginRegistry),
    SelectPlugin(String),
    SavesFound {
        plugin_id: String,
        saves: Vec<SaveFileInfo>,
    },
    RefreshSaves,
    EditPath,
    PathChanged(String),
    BrowsePath,
    PathBrowsed(Option<PathBuf>),
    ConfirmPath,
    CancelPath,
    OpenSave(PathBuf),
    Detected {
        request: u64,
        path: PathBuf,
        result: Result<Detected, Failure>,
    },
    ChoosePlugin {
        path: PathBuf,
        plugin_id: String,
    },
    Opened {
        request: u64,
        plugin_id: String,
        result: Result<OpenedDocument, Failure>,
    },
    Editor(UiMessage),
    Save,
    Saved {
        request: u64,
        result: Result<PathBuf, Failure>,
    },
    CloseEditor,
    GuardSave,
    GuardDiscard,
    CancelModal,
    CloseRequested(window::Id),
    Navigate(Screen),
    DismissInfo,
    OpenUrl(String),
}

#[derive(Debug)]
pub struct App {
    services: Option<Services>,
    fatal: Option<String>,
    settings: AppSettings,
    plugins: Plugins,
    screen: Screen,
    home: Home,
    editor: Option<Editor>,
    modal: Option<Modal>,
    status: Option<String>,
    /// Notice shown above the main area until dismissed.
    info: Option<String>,
    busy: Option<String>,
    /// Action to run once save request `.0` succeeds (from the unsaved guard).
    after_save: Option<(u64, Pending)>,
    /// Source of request numbers tying background results to their request.
    next_request: u64,
    /// The detect/open chain whose result is still wanted; others are stale.
    pending_open: Option<u64>,
}

impl App {
    /// Starts with plugins loading in the background (spec §5.2 Startup).
    pub fn new(
        services: Services,
        settings: AppSettings,
        warning: Option<String>,
    ) -> (Self, Task<Message>) {
        let engine = Arc::clone(&services.engine);
        let plugin_dir = settings.plugin_directory.clone();
        let app = Self {
            services: Some(services),
            fatal: None,
            settings,
            plugins: Plugins::Loading,
            screen: Screen::Home,
            home: Home::default(),
            editor: None,
            modal: None,
            status: warning,
            info: None,
            busy: Some("Loading plugins…".to_owned()),
            after_save: None,
            next_request: 0,
            pending_open: None,
        };
        let task = blocking(
            move || PluginRegistry::scan(&engine, &plugin_dir),
            Message::PluginsLoaded,
        );
        (app, task)
    }

    /// An app that can only show why it could not start.
    pub fn failed(reason: String, settings: AppSettings) -> Self {
        Self {
            services: None,
            fatal: Some(reason),
            settings,
            plugins: Plugins::Ready(PluginRegistry::default()),
            screen: Screen::Home,
            home: Home::default(),
            editor: None,
            modal: None,
            status: None,
            info: None,
            busy: None,
            after_save: None,
            next_request: 0,
            pending_open: None,
        }
    }

    pub fn title(&self) -> String {
        window_title(
            self.editor.as_ref().map(Editor::file_name).as_deref(),
            self.editor.as_ref().is_some_and(|e| e.dirty),
        )
    }

    pub fn screen(&self) -> &Screen {
        &self.screen
    }

    pub fn info(&self) -> Option<&str> {
        self.info.as_deref()
    }

    pub fn home(&self) -> &Home {
        &self.home
    }

    pub fn editor(&self) -> Option<&Editor> {
        self.editor.as_ref()
    }

    pub fn modal(&self) -> Option<&Modal> {
        self.modal.as_ref()
    }

    pub fn status(&self) -> Option<&str> {
        self.status.as_deref()
    }

    pub fn busy(&self) -> Option<&str> {
        self.busy.as_deref()
    }

    /// The request number of the open in progress, if any.
    pub fn open_request(&self) -> Option<u64> {
        self.pending_open
    }

    pub fn settings(&self) -> &AppSettings {
        &self.settings
    }

    pub fn registry(&self) -> Option<&PluginRegistry> {
        match &self.plugins {
            Plugins::Ready(registry) => Some(registry),
            Plugins::Loading => None,
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            window::close_requests().map(Message::CloseRequested),
            keyboard::listen().filter_map(|event| match event {
                keyboard::Event::KeyPressed { key, modifiers, .. }
                    if modifiers.command() && key == keyboard::Key::Character("s".into()) =>
                {
                    Some(Message::Save)
                }
                _ => None,
            }),
        ])
    }

    #[allow(clippy::too_many_lines)] // one arm per message keeps the flow readable
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::PluginsLoaded(registry) => {
                self.busy = None;
                let first = registry.loaded().next().map(|p| p.manifest.id.clone());
                self.plugins = Plugins::Ready(registry);
                first.map_or_else(Task::none, |id| self.select_plugin(id))
            }
            Message::SelectPlugin(id) => {
                self.screen = Screen::Home;
                if self.home.selected.as_deref() == Some(id.as_str()) {
                    return Task::none();
                }
                self.guarded(Pending::SelectPlugin(id))
            }
            Message::SavesFound { plugin_id, saves } => {
                if self.home.selected.as_deref() == Some(plugin_id.as_str()) {
                    self.home.saves = Saves::Found(saves);
                }
                Task::none()
            }
            Message::RefreshSaves => self.scan_saves(),
            Message::EditPath => {
                self.home.editing = Some(self.home.raw_dir.clone().unwrap_or_default());
                Task::none()
            }
            Message::PathChanged(text) => {
                if self.home.editing.is_some() {
                    self.home.editing = Some(text);
                }
                Task::none()
            }
            Message::BrowsePath => Task::perform(
                async {
                    rfd::AsyncFileDialog::new()
                        .pick_folder()
                        .await
                        .map(|handle| handle.path().to_path_buf())
                },
                Message::PathBrowsed,
            ),
            Message::PathBrowsed(Some(path)) => {
                self.home.editing = Some(path.display().to_string());
                Task::none()
            }
            Message::PathBrowsed(None) => Task::none(),
            Message::ConfirmPath => self.confirm_path(),
            Message::CancelPath => {
                self.home.editing = None;
                Task::none()
            }
            Message::OpenSave(path) => self.guarded(Pending::OpenFile(path)),
            Message::Detected {
                request,
                path,
                result,
            } => self.detected(request, path, result),
            Message::ChoosePlugin { path, plugin_id } => {
                self.modal = None;
                if let (Some(request), Some(plugin)) =
                    (self.pending_open, self.loaded_plugin(&plugin_id))
                {
                    self.start_open(request, plugin, path)
                } else {
                    self.pending_open = None;
                    Task::none()
                }
            }
            Message::Opened {
                request,
                plugin_id,
                result,
            } => {
                if self.pending_open != Some(request) {
                    // Superseded by a newer open, a plugin switch or a cancel.
                    return Task::none();
                }
                self.pending_open = None;
                self.busy = None;
                match result {
                    Ok(opened) => {
                        self.status = None;
                        self.editor = Some(Editor::new(opened));
                    }
                    Err(failure) => {
                        self.status = Some(failure.message.clone());
                        if failure.plugin_fault {
                            self.record_fault(&plugin_id);
                        }
                    }
                }
                Task::none()
            }
            Message::Editor(ui) => {
                let effect = self.editor.as_mut().map(|editor| editor.handle(ui));
                if effect == Some(EditorEffect::PluginFault)
                    && let Some(id) = self.editor.as_ref().map(|e| e.plugin_id.clone())
                {
                    self.record_fault(&id);
                }
                Task::none()
            }
            Message::Save => self.start_save(),
            Message::Saved { request, result } => self.saved(request, result),
            Message::CloseEditor => self.guarded(Pending::CloseEditor),
            Message::GuardSave => {
                if let Some(Modal::Unsaved { then }) = self.modal.take() {
                    let task = self.start_save();
                    // Keyed to the save in flight (new or already running).
                    if let Some(request) = self.editor.as_ref().and_then(|e| e.save_request) {
                        self.after_save = Some((request, then));
                    }
                    return task;
                }
                Task::none()
            }
            Message::GuardDiscard => match self.modal.take() {
                Some(Modal::Unsaved { then }) => self.perform(then),
                other => {
                    self.modal = other;
                    Task::none()
                }
            },
            Message::CancelModal => {
                if matches!(self.modal.take(), Some(Modal::ChoosePlugin { .. })) {
                    self.pending_open = None;
                }
                Task::none()
            }
            Message::CloseRequested(id) => self.guarded(Pending::Exit(id)),
            Message::Navigate(screen) => {
                self.screen = screen;
                Task::none()
            }
            Message::DismissInfo => {
                self.info = None;
                Task::none()
            }
            Message::OpenUrl(url) => {
                if let Err(e) = open::that_detached(&url) {
                    self.status = Some(format!("Could not open {url}: {e}"));
                }
                Task::none()
            }
        }
    }

    /// Runs `action` now, or asks first when the editor has unsaved changes.
    fn guarded(&mut self, action: Pending) -> Task<Message> {
        if self
            .editor
            .as_ref()
            .is_some_and(Editor::has_unsaved_changes)
        {
            self.modal = Some(Modal::Unsaved { then: action });
            Task::none()
        } else {
            self.perform(action)
        }
    }

    fn perform(&mut self, action: Pending) -> Task<Message> {
        match action {
            Pending::OpenFile(path) => {
                self.drop_editor();
                self.start_detect(path)
            }
            Pending::SelectPlugin(id) => self.select_plugin(id),
            Pending::CloseEditor => {
                self.drop_editor();
                Task::none()
            }
            Pending::Exit(id) => window::close(id),
        }
    }

    /// Closes the document. A save still in flight finishes on disk, but its
    /// result no longer applies to anything shown.
    fn drop_editor(&mut self) {
        if self.editor.take().is_some_and(|e| e.saving) {
            self.busy = None;
        }
    }

    fn next_request(&mut self) -> u64 {
        self.next_request += 1;
        self.next_request
    }

    fn select_plugin(&mut self, id: String) -> Task<Message> {
        self.drop_editor();
        if self.pending_open.take().is_some() {
            self.busy = None;
        }
        self.home.editing = None;
        self.home.raw_dir = self
            .loaded_plugin(&id)
            .and_then(|p| raw_save_dir(&self.settings, &p.manifest, Os::current()));
        self.home.selected = Some(id);
        self.scan_saves()
    }

    fn scan_saves(&mut self) -> Task<Message> {
        let (Some(services), Some(id)) = (&self.services, self.home.selected.clone()) else {
            return Task::none();
        };
        let Some(plugin) = self.loaded_plugin(&id) else {
            return Task::none();
        };
        match resolve_save_dir(self.home.raw_dir.as_deref(), &services.tokens) {
            Err(reason) => {
                self.home.saves = Saves::NotConfigured(reason);
                Task::none()
            }
            Ok(dir) => {
                self.home.saves = Saves::Scanning;
                let extensions = plugin.manifest.file_extensions.clone();
                blocking(
                    move || discover(&dir, &extensions),
                    move |saves| Message::SavesFound {
                        plugin_id: id.clone(),
                        saves,
                    },
                )
            }
        }
    }

    fn confirm_path(&mut self) -> Task<Message> {
        let (Some(raw), Some(id)) = (self.home.editing.take(), self.home.selected.clone()) else {
            return Task::none();
        };
        let raw = raw.trim().to_owned();
        self.settings
            .plugin_save_directories
            .insert(id, raw.clone());
        if let Some(services) = &self.services
            && let Err(e) = self.settings.save(&services.paths.settings_file())
        {
            tracing::warn!(error = %e, "could not save settings");
            self.status = Some(format!("Could not save settings: {e}"));
        }
        self.home.raw_dir = Some(raw);
        self.scan_saves()
    }

    fn start_detect(&mut self, path: PathBuf) -> Task<Message> {
        let (Some(services), Plugins::Ready(registry)) = (&self.services, &self.plugins) else {
            return Task::none();
        };
        let engine = Arc::clone(&services.engine);
        let registry = registry.clone();
        let request = self.next_request();
        self.pending_open = Some(request);
        self.busy = Some("Detecting…".to_owned());
        let target = path.clone();
        blocking(
            move || detect_file(&engine, &registry, &target),
            move |result| Message::Detected {
                request,
                path: path.clone(),
                result,
            },
        )
    }

    fn detected(
        &mut self,
        request: u64,
        path: PathBuf,
        result: Result<Detected, Failure>,
    ) -> Task<Message> {
        if self.pending_open != Some(request) {
            return Task::none();
        }
        self.busy = None;
        let detected = match result {
            Ok(detected) => detected,
            Err(failure) => {
                self.pending_open = None;
                self.status = Some(failure.message);
                return Task::none();
            }
        };
        for id in &detected.faulted {
            self.record_fault(id);
        }
        match detected.outcome {
            DetectOutcome::Open(plugin) => self.start_open(request, plugin, path),
            DetectOutcome::Choose(candidates) => {
                self.modal = Some(Modal::ChoosePlugin { path, candidates });
                Task::none()
            }
            DetectOutcome::NoMatch => {
                self.pending_open = None;
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                self.status = Some(format!("No plugin can open {name}"));
                Task::none()
            }
        }
    }

    fn start_open(
        &mut self,
        request: u64,
        plugin: Arc<LoadedPlugin>,
        path: PathBuf,
    ) -> Task<Message> {
        let Some(services) = &self.services else {
            return Task::none();
        };
        self.busy = Some(format!("Opening with {}…", plugin.manifest.name));
        let engine = Arc::clone(&services.engine);
        let tokens = Arc::clone(&services.tokens);
        let plugin_id = plugin.manifest.id.clone();
        blocking(
            move || open_document(&engine, &plugin, &path, &tokens),
            move |result| Message::Opened {
                request,
                plugin_id: plugin_id.clone(),
                result,
            },
        )
    }

    fn start_save(&mut self) -> Task<Message> {
        let Some(editor) = self.editor.as_mut() else {
            return Task::none();
        };
        if editor.saving || editor.crashed.is_some() {
            return Task::none();
        }
        self.next_request += 1;
        let request = self.next_request;
        editor.saving = true;
        editor.save_request = Some(request);
        self.busy = Some("Saving…".to_owned());
        let session = editor.session();
        let path = editor.path.clone();
        let backup_root = self.settings.backup_directory.clone();
        blocking(
            move || save_document(&session, &path, &backup_root),
            move |result| Message::Saved { request, result },
        )
    }

    fn saved(&mut self, request: u64, result: Result<PathBuf, Failure>) -> Task<Message> {
        let after = self
            .after_save
            .take_if(|(r, _)| *r == request)
            .map(|(_, pending)| pending);
        // The document this save belonged to may have been closed or replaced.
        let Some(editor) = self
            .editor
            .as_mut()
            .filter(|e| e.save_request == Some(request))
        else {
            return Task::none();
        };
        self.busy = None;
        editor.save_request = None;
        match result {
            Ok(backup) => {
                editor.saved();
                self.status = Some(format!("Saved. Backup: {}", backup.display()));
                // The file's size and modified time changed: refresh the list.
                let rescan = self.scan_saves();
                let then = after.map_or_else(Task::none, |pending| self.perform(pending));
                Task::batch([rescan, then])
            }
            Err(failure) => {
                editor.saving = false;
                self.status = Some(failure.message.clone());
                if failure.plugin_fault {
                    editor.crash(&failure);
                    let id = editor.plugin_id.clone();
                    self.record_fault(&id);
                }
                Task::none()
            }
        }
    }

    fn loaded_plugin(&self, id: &str) -> Option<Arc<LoadedPlugin>> {
        match self.registry()?.get(id)? {
            PluginEntry::Loaded(plugin) => Some(Arc::clone(plugin)),
            PluginEntry::Failed { .. } => None,
        }
    }

    fn record_fault(&mut self, id: &str) {
        if let Plugins::Ready(registry) = &mut self.plugins
            && registry.record_fault(id)
        {
            self.status = Some(format!(
                "Plugin '{id}' was disabled after repeated crashes. Restart SaveForge to re-enable it."
            ));
        }
    }
}

/// Runs `work` on tokio's blocking pool and maps its result to a message.
fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
    to_message: impl Fn(T) -> Message + Send + 'static,
) -> Task<Message> {
    Task::perform(
        async move {
            tokio::task::spawn_blocking(work)
                .await
                .expect("blocking task panicked")
        },
        to_message,
    )
}

/// Builds the app's services and runs the iced event loop.
pub fn run() -> iced::Result {
    // Must happen before any thread starts: `time` refuses to read the local
    // offset on Unix once the process is multi-threaded.
    let local_offset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
    let paths = AppPaths::detect()
        .unwrap_or_else(|| AppPaths::under(std::env::temp_dir().join("SaveForge")));
    let _log_guard = crate::logging::init(&paths);
    crate::logging::install_panic_hook(paths.crash_log());
    let loaded = AppSettings::load(&paths.settings_file(), AppSettings::defaults(&paths));
    let settings = loaded.settings;
    let warning = loaded.warning;

    iced::application(
        move || boot(&paths, settings.clone(), warning.clone(), local_offset),
        App::update,
        App::view,
    )
    .title(App::title)
    .theme(|_: &App| crate::theme::theme())
    .subscription(App::subscription)
    .window(window::Settings {
        size: iced::Size::new(1200.0, 750.0),
        min_size: Some(iced::Size::new(900.0, 600.0)),
        position: window::Position::Centered,
        exit_on_close_request: false,
        ..window::Settings::default()
    })
    .run()
}

fn boot(
    paths: &AppPaths,
    settings: AppSettings,
    warning: Option<String>,
    local_offset: UtcOffset,
) -> (App, Task<Message>) {
    let cache = paths.wasm_cache_dir();
    match PluginEngine::new(saveforge_host::HostLimits::default(), Some(&cache)) {
        Ok(engine) => App::new(
            Services {
                engine: Arc::new(engine),
                paths: paths.clone(),
                tokens: Arc::new(TokenEnv::detect()),
                local_offset,
            },
            settings,
            warning,
        ),
        Err(e) => {
            tracing::error!(error = %e, "plugin engine failed to start");
            (
                App::failed(format!("SaveForge could not start: {e}"), settings),
                Task::none(),
            )
        }
    }
}
