//! The shell layout (spec §5.1, design `SaveEditor.html`): plugin rail,
//! saves pane, main pane, status bar, toast and modals.

use iced::widget::{
    Column, Id, Row, button, center, column, container, image, opaque, row, scrollable, space,
    stack, text, text_input, tooltip,
};
use iced::{Alignment, Element, Length, Padding};
use saveforge_core::discovery::{ListedSave, SaveGroup};
use saveforge_host::{HOST_VERSION, LoadedPlugin, PluginEntry};
use time::UtcOffset;

use super::{App, Message, Modal, Pending, Plugins, Screen, settings_view};
use crate::components::{Size, badge, btn, caption, dot, glyph, hline, icon_btn, vline};
use crate::fonts;
use crate::format::{format_modified, human_size};
use crate::home::{Saves, resolve_save_dir};
use crate::icons::{Icon, icon};
use crate::render::render;
use crate::theme::{self, Tone};

/// Ids of the rail's tool buttons, for tests (the buttons show icons only).
pub const RAIL_STORE: &str = "rail-store";
pub const RAIL_SETTINGS: &str = "rail-settings";
pub const RAIL_ABOUT: &str = "rail-about";

impl App {
    pub fn view(&self) -> Element<'_, Message> {
        let mut body = Row::new().push(self.rail()).push(vline());
        if self.screen == Screen::Home {
            body = body.push(self.save_panel()).push(vline());
        }
        let mut page = Column::new();
        if let Some(info) = &self.info {
            page = page.push(info_bar(info));
        }
        let page = page
            .push(body.push(self.main_pane()).height(Length::Fill))
            .push(hline())
            .push(self.status_bar());
        let mut layers = stack![container(page).style(theme::bg1)];
        if let Some(status) = &self.status {
            layers = layers.push(toast(status));
        }
        if let Some(modal) = &self.modal {
            layers = layers.push(opaque(
                center(self.modal_view(modal))
                    .style(theme::backdrop)
                    .width(Length::Fill)
                    .height(Length::Fill),
            ));
        }
        layers.into()
    }

    // ───────────── rail ─────────────

    fn rail(&self) -> Element<'_, Message> {
        let mut col = Column::new()
            .spacing(4)
            .height(Length::Fill)
            .align_x(Alignment::Center);
        match &self.plugins {
            Plugins::Loading => col = col.push(caption("…")),
            Plugins::Ready(registry) => {
                col = col.extend(registry.entries().iter().map(|e| self.rail_button(e)));
            }
        }
        let store_active = matches!(self.screen, Screen::Store | Screen::PluginDetail(_));
        col = col
            .push(container(space()).width(32).height(1).style(theme::line))
            .push(rail_tool(
                Icon::Plus,
                "Browse plugins",
                RAIL_STORE,
                Screen::Store,
                store_active,
                true,
            ))
            .push(space::vertical())
            .push(rail_tool(
                Icon::Cog,
                "Settings",
                RAIL_SETTINGS,
                Screen::Settings,
                self.screen == Screen::Settings,
                false,
            ))
            .push(rail_tool(
                Icon::Info,
                "About SaveForge",
                RAIL_ABOUT,
                Screen::About,
                self.screen == Screen::About,
                false,
            ));
        container(col)
            .padding([10, 0])
            .width(Length::Fixed(theme::RAIL_WIDTH))
            .height(Length::Fill)
            .style(theme::bg0)
            .into()
    }

    fn rail_button<'a>(&'a self, entry: &'a PluginEntry) -> Element<'a, Message> {
        let id = entry.id();
        let selected = self.screen == Screen::Home && self.home.selected.as_deref() == Some(id);
        let (face, tip, on_press): (Element<'a, Message>, String, Option<Message>) = match entry {
            PluginEntry::Loaded(plugin) => {
                let tip = if selected {
                    format!(
                        "{}  v{} — {}",
                        plugin.manifest.name,
                        plugin.manifest.version,
                        self.save_summary()
                    )
                } else {
                    format!("{}  v{}", plugin.manifest.name, plugin.manifest.version)
                };
                (
                    plugin_face(plugin, 40.0),
                    tip,
                    Some(Message::SelectPlugin(id.to_owned())),
                )
            }
            PluginEntry::Failed { reason, .. } => (
                glyph(id, id, 40.0),
                format!("{id}: load failed — {reason}"),
                None,
            ),
        };
        let marker: Element<'a, Message> = if selected {
            container(space())
                .width(3)
                .height(24)
                .style(theme::marker)
                .into()
        } else {
            space().width(3).into()
        };
        let face = button(face)
            .padding(0)
            .style(theme::bare)
            .on_press_maybe(on_press);
        row![
            space().width(Length::Fill),
            marker,
            space().width(7),
            tip_right(face, tip),
            space().width(14),
        ]
        .width(Length::Fill)
        .align_y(Alignment::Center)
        .into()
    }

    fn save_summary(&self) -> String {
        match &self.home.saves {
            Saves::NotConfigured(_) => "path not set".to_owned(),
            Saves::Scanning => "scanning…".to_owned(),
            Saves::Found(_) if self.home.saves.count() == 1 => "1 save".to_owned(),
            Saves::Found(_) => format!("{} saves", self.home.saves.count()),
        }
    }

    // ───────────── saves pane ─────────────

    fn selected_plugin(&self) -> Option<&LoadedPlugin> {
        let Plugins::Ready(registry) = &self.plugins else {
            return None;
        };
        match registry.get(self.home.selected.as_deref()?)? {
            PluginEntry::Loaded(plugin) => Some(plugin),
            PluginEntry::Failed { .. } => None,
        }
    }

    fn save_panel(&self) -> Element<'_, Message> {
        let body: Element<'_, Message> = match (&self.plugins, self.selected_plugin()) {
            (Plugins::Loading, _) => container(caption("Loading plugins…")).padding(14).into(),
            (Plugins::Ready(_), None) => column![
                text("No plugins installed").size(13),
                caption(format!(
                    "Plugins are loaded from {}",
                    self.settings.plugin_directory.display()
                )),
            ]
            .spacing(8)
            .padding(14)
            .into(),
            (Plugins::Ready(_), Some(plugin)) => column![
                plugin_header(plugin),
                hline(),
                self.folder_row(),
                hline(),
                self.saves_body(),
            ]
            .height(Length::Fill)
            .into(),
        };
        container(body)
            .width(Length::Fixed(theme::PANEL_WIDTH))
            .height(Length::Fill)
            .style(theme::bg1)
            .into()
    }

    /// The save folder strip, or the path editor while it is open.
    fn folder_row(&self) -> Element<'_, Message> {
        if let Some(editing) = &self.home.editing {
            return container(self.path_editor(editing))
                .padding([10, 14])
                .style(theme::bg0)
                .into();
        }
        let path = self.home.raw_dir.as_deref().unwrap_or("No folder set");
        container(
            row![
                icon(Icon::Folder, 11.0, theme::TEXT_FAINT),
                caption(path)
                    .width(Length::Fill)
                    .wrapping(text::Wrapping::Glyph),
                tip_below(
                    icon_btn(Icon::Pencil, 20.0).on_press(Message::EditPath),
                    "Change folder",
                ),
                tip_below(
                    icon_btn(Icon::Refresh, 20.0).on_press(Message::RefreshSaves),
                    "Rescan",
                ),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .padding([6, 14])
        .width(Length::Fill)
        .style(theme::bg0)
        .into()
    }

    fn saves_body(&self) -> Element<'_, Message> {
        match &self.home.saves {
            Saves::NotConfigured(reason) => column![
                text("No save folder").size(13),
                caption(reason.as_str()),
                btn("Set path", Some(Icon::Folder), Size::Small).on_press(Message::EditPath),
            ]
            .spacing(8)
            .padding(14)
            .into(),
            Saves::Scanning => container(caption("Scanning…")).padding(14).into(),
            Saves::Found(groups) if groups.is_empty() => column![
                caption("No saves found"),
                btn("Refresh", Some(Icon::Refresh), Size::Small).on_press(Message::RefreshSaves),
            ]
            .spacing(8)
            .padding(14)
            .into(),
            Saves::Found(groups) => scrollable(
                column![
                    container(caption(format!("SAVES · {}", self.home.saves.count())))
                        .padding(Padding::new(0.0).top(8).left(6).bottom(2)),
                    self.save_list(groups),
                ]
                .padding([0, 8]),
            )
            .style(theme::scroll)
            .height(Length::Fill)
            .into(),
        }
    }

    /// The saves, under a foldable header per group when the plugin groups
    /// them.
    fn save_list<'a>(&'a self, groups: &'a [SaveGroup]) -> Column<'a, Message> {
        let grouped = groups.iter().any(|g| g.name.is_some());
        let mut col = Column::new()
            .spacing(4)
            .padding(Padding::new(0.0).bottom(8));
        for group in groups {
            if grouped {
                let (name, key) = match &group.name {
                    Some(name) => (name.as_str(), name.clone()),
                    None => ("Other", String::new()),
                };
                let collapsed = self.home.collapsed.contains(&key);
                col = col.push(
                    button(
                        row![
                            caption(if collapsed { "▸" } else { "▾" }),
                            text(name)
                                .size(12)
                                .font(fonts::MONO_MEDIUM)
                                .width(Length::Fill),
                            caption(group.saves.len()),
                        ]
                        .spacing(6)
                        .align_y(Alignment::Center),
                    )
                    .padding([4, 6])
                    .width(Length::Fill)
                    .style(theme::group_header)
                    .on_press(Message::ToggleSaveGroup(key)),
                );
                if collapsed {
                    continue;
                }
            }
            col = col.extend(group.saves.iter().map(|save| self.save_tile(save)));
        }
        col
    }

    fn save_tile<'a>(&'a self, save: &'a ListedSave) -> Element<'a, Message> {
        let offset = self.local_offset();
        let file = &save.file;
        let open = self.editor.as_ref().filter(|e| e.path == file.path);
        let mut tags = row![].spacing(6).align_y(Alignment::Center);
        if let Some(editor) = open {
            tags = tags.push(badge("OPEN", Tone::Amber));
            if editor.has_unsaved_changes() {
                tags = tags.push(space::horizontal()).push(dot(theme::AMBER, 6.0));
            }
        }
        let mut body = column![].spacing(4);
        if open.is_some() {
            body = body.push(tags);
        }
        body = body
            .push(text(save.label.as_str()).size(12).font(fonts::MONO_MEDIUM))
            .push(
                row![
                    caption(format_modified(file.modified, offset)).width(Length::Fill),
                    caption(human_size(file.size_bytes)),
                ]
                .spacing(6),
            );
        button(body)
            .padding([10, 12])
            .width(Length::Fill)
            .style(theme::tile(open.is_some()))
            .on_press(Message::OpenSave(file.path.clone()))
            .into()
    }

    fn path_editor<'a>(&'a self, editing: &'a str) -> Element<'a, Message> {
        let hint = match &self.services {
            Some(services) => match resolve_save_dir(Some(editing), &services.tokens) {
                Ok(path) => caption(format!("→ {}", path.display())),
                Err(reason) => text(reason).size(10).style(theme::error_text),
            },
            None => text(""),
        };
        column![
            text_input("%APPDATA%/Game/Saves", editing)
                .size(11)
                .padding([4, 8])
                .style(theme::input)
                .on_input(Message::PathChanged)
                .on_submit(Message::ConfirmPath),
            row![
                btn("Browse…", Some(Icon::Folder), Size::Small).on_press(Message::BrowsePath),
                space::horizontal(),
                btn("Cancel", None, Size::Small)
                    .style(theme::btn_ghost)
                    .on_press(Message::CancelPath),
                btn("Save path", None, Size::Small)
                    .style(theme::btn_primary)
                    .on_press(Message::ConfirmPath),
            ]
            .spacing(6),
            hint.wrapping(text::Wrapping::Glyph),
        ]
        .spacing(8)
        .into()
    }

    // ───────────── main pane ─────────────

    fn main_pane(&self) -> Element<'_, Message> {
        if let Some(reason) = &self.fatal {
            return center(text(reason.as_str()).style(theme::error_text)).into();
        }
        match &self.screen {
            Screen::Home => self.home_pane(),
            Screen::Store => padded(self.store_list()),
            Screen::PluginDetail(id) => padded(self.plugin_detail(id)),
            Screen::Settings => padded(self.settings_view()),
            Screen::About => padded(settings_view::about()),
        }
    }

    fn home_pane(&self) -> Element<'_, Message> {
        let Some(editor) = &self.editor else {
            let hint = self
                .busy
                .as_deref()
                .unwrap_or("Select a save on the left to edit it.");
            return center(text(hint).size(12).style(theme::faint)).into();
        };
        let mut col = column![self.editor_toolbar(editor), hline()];
        if let Some(reason) = &editor.crashed {
            return col
                .push(center(
                    container(
                        column![
                            row![
                                icon(Icon::Warn, 14.0, theme::RED_TEXT),
                                text("THE PLUGIN CRASHED")
                                    .size(18)
                                    .font(fonts::PIXEL)
                                    .style(theme::error_text),
                            ]
                            .spacing(8)
                            .align_y(Alignment::Center),
                            text(reason.as_str()).size(11).style(theme::muted),
                            caption("The file on disk was not changed."),
                            btn("Back", None, Size::Small)
                                .style(theme::btn_danger)
                                .on_press(Message::CloseEditor),
                        ]
                        .spacing(10),
                    )
                    .padding(16)
                    .max_width(520)
                    .style(theme::danger_box),
                ))
                .into();
        }
        if editor.has_unsaved_changes() {
            col = col.push(unsaved_bar(editor.saving));
        }
        col.push(
            scrollable(
                container(render(&editor.tree, &editor.ui).map(Message::Editor)).padding([20, 28]),
            )
            .style(theme::scroll)
            .height(Length::Fill),
        )
        .into()
    }

    fn editor_toolbar<'a>(&'a self, editor: &'a super::Editor) -> Element<'a, Message> {
        let detail = match self.listed_save(&editor.path) {
            Some(save) => format!(
                "{} · {} · modified {}",
                save.file.file_name,
                human_size(save.file.size_bytes),
                format_modified(save.file.modified, self.local_offset())
            ),
            None => editor.path.display().to_string(),
        };
        let title = self
            .listed_save(&editor.path)
            .map_or_else(|| editor.file_name(), |s| s.label.clone());
        container(
            row![
                column![
                    text(title).size(19).font(fonts::PIXEL),
                    caption(detail).wrapping(text::Wrapping::None),
                ]
                .spacing(2)
                .width(Length::Fill)
                .clip(true),
                btn("Close", Some(Icon::Close), Size::Small).on_press(Message::CloseEditor),
                container(space()).width(1).height(20).style(theme::line),
                btn("Save", Some(Icon::Save), Size::Small)
                    .style(theme::btn_primary)
                    .on_press_maybe((!editor.saving).then_some(Message::Save)),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .padding([0, 10])
        .height(theme::TOOLBAR_HEIGHT)
        .align_y(Alignment::Center)
        .width(Length::Fill)
        .style(theme::bg2)
        .into()
    }

    /// The listed save at `path`, for its label, size and date.
    fn listed_save(&self, path: &std::path::Path) -> Option<&ListedSave> {
        let Saves::Found(groups) = &self.home.saves else {
            return None;
        };
        groups
            .iter()
            .flat_map(|g| &g.saves)
            .find(|s| s.file.path == path)
    }

    fn local_offset(&self) -> UtcOffset {
        self.services
            .as_ref()
            .map_or(UtcOffset::UTC, |s| s.local_offset)
    }

    // ───────────── status bar ─────────────

    fn status_bar(&self) -> Element<'_, Message> {
        let mut bar = row![].spacing(16).align_y(Alignment::Center);
        let (led, state) = match (&self.fatal, &self.busy) {
            (Some(_), _) => (theme::RED, "ERROR".to_owned()),
            (None, Some(busy)) => (theme::AMBER, busy.to_uppercase()),
            (None, None) => (theme::GREEN, "READY".to_owned()),
        };
        bar = bar.push(
            row![dot(led, 6.0), status_text(state)]
                .spacing(6)
                .align_y(Alignment::Center),
        );
        if self.screen == Screen::Home
            && let Some(plugin) = self.selected_plugin()
        {
            bar = bar.push(labelled_status(
                "PLUGIN",
                plugin.manifest.name.to_uppercase(),
                format!(" v{}", plugin.manifest.version),
            ));
        }
        if let Some(editor) = &self.editor {
            bar = bar.push(labelled_status(
                "SAVE",
                editor.file_name().to_uppercase(),
                String::new(),
            ));
        }
        bar = bar.push(space::horizontal());
        if let Some(editor) = &self.editor {
            bar = bar.push(if editor.saving {
                status_text("SAVING…").style(theme::heading)
            } else if editor.has_unsaved_changes() {
                status_text("● UNSAVED").style(theme::heading)
            } else {
                status_text("✓ SAVED").style(theme::success)
            });
        }
        bar = bar.push(status_text(format!("SAVEFORGE {}", *HOST_VERSION)));
        container(bar)
            .padding([0, 10])
            .width(Length::Fill)
            .height(Length::Fixed(theme::STATUS_HEIGHT))
            .align_y(Alignment::Center)
            .style(theme::status_bar)
            .into()
    }

    // ───────────── modals ─────────────

    fn modal_view<'a>(&'a self, modal: &'a Modal) -> Element<'a, Message> {
        let (title, body): (&str, Column<'a, Message>) = match modal {
            Modal::ChoosePlugin { path, candidates } => {
                ("Choose a plugin", choose_plugin(path, candidates))
            }
            Modal::Unsaved { then } => {
                let file = self
                    .editor
                    .as_ref()
                    .map(super::Editor::file_name)
                    .unwrap_or_default();
                let action = match then {
                    Pending::Exit(_) => "quitting",
                    Pending::OpenFile(_) => "opening another file",
                    Pending::SelectPlugin(_) => "switching plugin",
                    Pending::CloseEditor => "closing",
                };
                (
                    "Unsaved changes",
                    column![
                        text(format!("Save changes to {file} before {action}?")).size(12),
                        row![
                            space::horizontal(),
                            btn("Cancel", None, Size::Small)
                                .style(theme::btn_ghost)
                                .on_press(Message::CancelModal),
                            btn("Discard", None, Size::Small)
                                .style(theme::btn_danger)
                                .on_press(Message::GuardDiscard),
                            btn("Save", Some(Icon::Save), Size::Small)
                                .style(theme::btn_primary)
                                .on_press(Message::GuardSave),
                        ]
                        .spacing(8),
                    ]
                    .spacing(16),
                )
            }
        };
        let header = container(
            row![
                text(title.to_uppercase())
                    .size(18)
                    .font(fonts::PIXEL)
                    .style(theme::heading)
                    .width(Length::Fill),
                icon_btn(Icon::Close, 24.0).on_press(Message::CancelModal),
            ]
            .align_y(Alignment::Center),
        )
        .padding([8, 16])
        .style(theme::bg1);
        container(column![header, hline(), container(body).padding(16)])
            .max_width(460)
            .style(theme::modal)
            .into()
    }
}

/// The body of the "several plugins can open this file" modal.
fn choose_plugin<'a>(
    path: &'a std::path::Path,
    candidates: &'a [std::sync::Arc<LoadedPlugin>],
) -> Column<'a, Message> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut col = column![
        text(format!("Several plugins can open {name}."))
            .size(12)
            .style(theme::muted)
    ]
    .spacing(8);
    col = col.extend(candidates.iter().map(|plugin| {
        button(
            row![
                glyph(&plugin.manifest.id, &plugin.manifest.name, 28.0),
                column![
                    text(plugin.manifest.name.as_str()).size(13),
                    caption(format!(
                        "v{} · {}",
                        plugin.manifest.version, plugin.manifest.author
                    )),
                ]
                .spacing(2),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        )
        .padding([8, 10])
        .width(Length::Fill)
        .style(theme::tile(false))
        .on_press(Message::ChoosePlugin {
            path: path.to_path_buf(),
            plugin_id: plugin.manifest.id.clone(),
        })
        .into()
    }));
    col.push(
        row![
            space::horizontal(),
            btn("Cancel", None, Size::Small)
                .style(theme::btn_ghost)
                .on_press(Message::CancelModal),
        ]
        .padding(Padding::new(0.0).top(4)),
    )
}

/// A main-pane page with the design's body padding.
fn padded(content: Element<'_, Message>) -> Element<'_, Message> {
    container(content)
        .padding([20, 28])
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// The selected plugin's header in the saves pane.
fn plugin_header(plugin: &LoadedPlugin) -> Element<'_, Message> {
    container(
        row![
            plugin_face(plugin, 34.0),
            column![
                text(plugin.manifest.name.as_str())
                    .size(20)
                    .font(fonts::PIXEL)
                    .wrapping(text::Wrapping::None),
                caption(format!(
                    "v{} · {}",
                    plugin.manifest.version, plugin.manifest.author
                )),
            ]
            .spacing(2)
            .width(Length::Fill)
            .clip(true),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    )
    .padding([12, 14])
    .into()
}

/// The plugin's own icon when it ships one, else its monogram glyph.
fn plugin_face<'a>(plugin: &LoadedPlugin, size: f32) -> Element<'a, Message> {
    match &plugin.manifest.icon {
        Some(file) => image(image::Handle::from_path(plugin.dir.join(file)))
            .width(size)
            .height(size)
            .into(),
        None => glyph(&plugin.manifest.id, &plugin.manifest.name, size),
    }
}

/// A rail tool (store, settings, about): an icon with a tooltip.
fn rail_tool<'a>(
    lead: Icon,
    tip: &'a str,
    id: &'static str,
    screen: Screen,
    active: bool,
    framed: bool,
) -> Element<'a, Message> {
    let color = if active {
        theme::AMBER
    } else {
        theme::TEXT_DIM
    };
    let button = button(center(icon(lead, 16.0, color)))
        .width(40)
        .height(40)
        .padding(0)
        .style(theme::rail_tool(active, framed))
        .on_press(Message::Navigate(screen));
    container(tip_right(button, tip.to_owned()))
        .id(Id::new(id))
        .into()
}

fn tip_right<'a>(
    content: impl Into<Element<'a, Message>>,
    tip: impl text::IntoFragment<'a>,
) -> Element<'a, Message> {
    tooltip(
        content,
        container(text(tip).size(11)).padding([4, 8]),
        tooltip::Position::Right,
    )
    .gap(6)
    .style(theme::tooltip)
    .into()
}

fn tip_below<'a>(content: impl Into<Element<'a, Message>>, tip: &'a str) -> Element<'a, Message> {
    tooltip(
        content,
        container(text(tip).size(11)).padding([4, 8]),
        tooltip::Position::Bottom,
    )
    .gap(4)
    .style(theme::tooltip)
    .into()
}

fn status_text<'a>(label: impl text::IntoFragment<'a>) -> text::Text<'a> {
    text(label).size(10)
}

/// `PLUGIN <amber name> v1.0`.
fn labelled_status(label: &str, value: String, suffix: String) -> Element<'_, Message> {
    row![
        status_text(label),
        status_text(value).style(theme::heading),
        status_text(suffix),
    ]
    .spacing(4)
    .into()
}

/// The amber strip shown while the open save has unsaved changes.
fn unsaved_bar<'a>(saving: bool) -> Element<'a, Message> {
    container(
        row![
            dot(theme::AMBER, 8.0),
            text("Unsaved changes")
                .size(11)
                .font(fonts::MONO_MEDIUM)
                .style(theme::heading),
            text("· a backup is made when you save")
                .size(11)
                .style(theme::muted)
                .width(Length::Fill),
            btn("Save to file", None, Size::Small)
                .style(theme::btn_primary)
                .on_press_maybe((!saving).then_some(Message::Save)),
        ]
        .spacing(12)
        .align_y(Alignment::Center),
    )
    .padding([8, 14])
    .width(Length::Fill)
    .style(theme::notice_bar)
    .into()
}

fn info_bar(info: &str) -> Element<'_, Message> {
    container(
        row![
            icon(Icon::Download, 12.0, theme::AMBER),
            text(info).size(11).width(Length::Fill),
            btn("Dismiss", None, Size::Small)
                .style(theme::btn_ghost)
                .on_press(Message::DismissInfo),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    )
    .padding([6, 14])
    .width(Length::Fill)
    .style(theme::notice_bar)
    .into()
}

/// The last status message, bottom right above the status bar.
fn toast(status: &str) -> Element<'_, Message> {
    let card = container(
        row![
            container(space())
                .width(3)
                .height(Length::Fill)
                .style(theme::toast_edge(theme::AMBER)),
            row![
                text("ⓘ")
                    .size(11)
                    .font(fonts::MONO_BOLD)
                    .style(theme::heading),
                text(status).size(11).width(Length::Shrink),
                icon_btn(Icon::Close, 20.0).on_press(Message::DismissStatus),
            ]
            .spacing(8)
            .padding([8, 10])
            .align_y(Alignment::Center),
        ]
        .height(Length::Shrink),
    )
    .max_width(480)
    .style(theme::toast);
    container(opaque(card))
        .padding(
            Padding::new(0.0)
                .right(24)
                .bottom(theme::STATUS_HEIGHT + 22.0),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::End)
        .align_y(Alignment::End)
        .into()
}
