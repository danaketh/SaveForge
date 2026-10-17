//! The shell layout (spec §5.1): rail, save panel, main pane, status bar and
//! modals.

use iced::widget::{
    Column, button, center, column, container, image, opaque, row, scrollable, space, stack, text,
    text_input, tooltip,
};
use iced::{Alignment, Element, Length};
use saveforge_host::PluginEntry;
use time::UtcOffset;

use super::{App, Message, Modal, Pending, Plugins, Screen, settings_view};
use crate::format::{format_modified, human_size};
use crate::home::{Saves, resolve_save_dir};
use crate::render::render;
use crate::theme;

impl App {
    pub fn view(&self) -> Element<'_, Message> {
        let mut body = row![self.rail()];
        if self.screen == Screen::Home {
            body = body.push(self.save_panel());
        }
        let mut page = Column::new();
        if let Some(info) = &self.info {
            page = page.push(info_bar(info));
        }
        let page = page
            .push(body.push(self.main_pane()).height(Length::Fill))
            .push(self.status_bar());
        match &self.modal {
            Some(modal) => stack![
                page,
                opaque(
                    center(self.modal_view(modal))
                        .style(theme::backdrop)
                        .width(Length::Fill)
                        .height(Length::Fill)
                ),
            ]
            .into(),
            None => page.into(),
        }
    }

    fn rail(&self) -> Element<'_, Message> {
        let mut buttons: Vec<Element<'_, Message>> = match &self.plugins {
            Plugins::Loading => vec![text("…").into()],
            Plugins::Ready(registry) => registry
                .entries()
                .iter()
                .map(|entry| self.rail_button(entry))
                .collect(),
        };
        buttons.push(space::vertical().into());
        let store_active = matches!(self.screen, Screen::Store | Screen::PluginDetail(_));
        buttons.extend([
            nav_button(
                "Store",
                "Browse and install plugins",
                Screen::Store,
                store_active,
            ),
            nav_button(
                "Settings",
                "Catalog, folders and updates",
                Screen::Settings,
                self.screen == Screen::Settings,
            ),
            nav_button(
                "About",
                "Versions and licences",
                Screen::About,
                self.screen == Screen::About,
            ),
        ]);
        container(
            Column::with_children(buttons)
                .spacing(8)
                .height(Length::Fill)
                .align_x(Alignment::Center),
        )
        .padding([12, 0])
        .width(Length::Fixed(theme::RAIL_WIDTH))
        .height(Length::Fill)
        .align_x(Alignment::Center)
        .style(theme::rail)
        .into()
    }

    fn rail_button<'a>(&'a self, entry: &'a PluginEntry) -> Element<'a, Message> {
        let id = entry.id();
        let selected = self.screen == Screen::Home && self.home.selected.as_deref() == Some(id);
        let (face, tip, on_press): (Element<'a, Message>, String, Option<Message>) = match entry {
            PluginEntry::Loaded(plugin) => {
                let face = match &plugin.manifest.icon {
                    Some(icon) => image(image::Handle::from_path(plugin.dir.join(icon)))
                        .width(28)
                        .height(28)
                        .into(),
                    None => initial(&plugin.manifest.name),
                };
                let tip = if selected {
                    format!("{} — {}", plugin.manifest.name, self.save_summary())
                } else {
                    plugin.manifest.name.clone()
                };
                (face, tip, Some(Message::SelectPlugin(id.to_owned())))
            }
            PluginEntry::Failed { reason, .. } => {
                (initial(id), format!("{id}: load failed — {reason}"), None)
            }
        };
        let button = button(center(face))
            .width(40)
            .height(40)
            .padding(0)
            .style(theme::rail_button(selected))
            .on_press_maybe(on_press);
        tooltip(button, text(tip), tooltip::Position::Right).into()
    }

    fn save_summary(&self) -> String {
        match &self.home.saves {
            Saves::NotConfigured(_) => "path not set".to_owned(),
            Saves::Scanning => "scanning…".to_owned(),
            Saves::Found(saves) if saves.len() == 1 => "1 save".to_owned(),
            Saves::Found(saves) => format!("{} saves", saves.len()),
        }
    }

    fn save_panel(&self) -> Element<'_, Message> {
        container(scrollable(self.save_panel_body().padding(12)))
            .width(Length::Fixed(theme::PANEL_WIDTH))
            .height(Length::Fill)
            .style(theme::panel)
            .into()
    }

    fn save_panel_body(&self) -> Column<'_, Message> {
        let registry = match &self.plugins {
            Plugins::Loading => return column![text("Loading plugins…").style(theme::muted)],
            Plugins::Ready(registry) => registry,
        };
        let Some(plugin) = self
            .home
            .selected
            .as_deref()
            .and_then(|id| registry.get(id))
            .and_then(|entry| match entry {
                PluginEntry::Loaded(plugin) => Some(plugin),
                PluginEntry::Failed { .. } => None,
            })
        else {
            return column![
                text("No plugins installed").size(16),
                text(format!(
                    "Plugins are loaded from {}",
                    self.settings.plugin_directory.display()
                ))
                .size(12)
                .style(theme::muted),
            ]
            .spacing(8);
        };

        let mut col = column![text(plugin.manifest.name.as_str()).size(16)].spacing(10);
        if let Some(editing) = &self.home.editing {
            return col.push(self.path_editor(editing));
        }
        col = col.push(
            row![
                text(self.home.raw_dir.as_deref().unwrap_or("No folder set"))
                    .size(12)
                    .style(theme::muted)
                    .width(Length::Fill),
                button(text("Edit").size(12)).on_press(Message::EditPath),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        );
        match &self.home.saves {
            Saves::NotConfigured(reason) => col
                .push(text("No save folder").size(15))
                .push(text(reason.as_str()).size(12).style(theme::muted))
                .push(button("Set path").on_press(Message::EditPath)),
            Saves::Scanning => col.push(text("Scanning…").style(theme::muted)),
            Saves::Found(saves) if saves.is_empty() => col
                .push(text("No saves found").style(theme::muted))
                .push(button(text("Refresh").size(12)).on_press(Message::RefreshSaves)),
            Saves::Found(saves) => {
                let offset = self
                    .services
                    .as_ref()
                    .map_or(UtcOffset::UTC, |s| s.local_offset);
                let open_path = self.editor.as_ref().map(|e| &e.path);
                col = col.push(
                    row![
                        text(self.save_summary())
                            .size(12)
                            .style(theme::muted)
                            .width(Length::Fill),
                        button(text("Refresh").size(12)).on_press(Message::RefreshSaves),
                    ]
                    .align_y(Alignment::Center),
                );
                col.extend(saves.iter().map(|save| {
                    button(
                        column![
                            text(save.file_name.as_str()),
                            text(format!(
                                "{} · {}",
                                format_modified(save.modified, offset),
                                human_size(save.size_bytes)
                            ))
                            .size(12)
                            .style(theme::muted),
                        ]
                        .spacing(2),
                    )
                    .width(Length::Fill)
                    .style(theme::save_row(open_path == Some(&save.path)))
                    .on_press(Message::OpenSave(save.path.clone()))
                    .into()
                }))
            }
        }
    }

    fn path_editor<'a>(&'a self, editing: &'a str) -> Element<'a, Message> {
        let hint = match &self.services {
            Some(services) => match resolve_save_dir(Some(editing), &services.tokens) {
                Ok(path) => text(format!("→ {}", path.display())).style(theme::muted),
                Err(reason) => text(reason).style(theme::error_text),
            },
            None => text(""),
        };
        column![
            text_input("%APPDATA%/Game/Saves", editing)
                .on_input(Message::PathChanged)
                .on_submit(Message::ConfirmPath),
            row![
                button(text("Browse…").size(12)).on_press(Message::BrowsePath),
                space::horizontal(),
                button(text("Cancel").size(12))
                    .style(button::secondary)
                    .on_press(Message::CancelPath),
                button(text("Save path").size(12)).on_press(Message::ConfirmPath),
            ]
            .spacing(6),
            hint.size(12),
        ]
        .spacing(8)
        .into()
    }

    fn main_pane(&self) -> Element<'_, Message> {
        let content: Element<'_, Message> = if let Some(reason) = &self.fatal {
            center(text(reason.as_str()).style(theme::error_text)).into()
        } else {
            match &self.screen {
                Screen::Home => self.home_pane(),
                Screen::Store => self.store_list(),
                Screen::PluginDetail(id) => self.plugin_detail(id),
                Screen::Settings => self.settings_view(),
                Screen::About => settings_view::about(),
            }
        };
        container(content)
            .padding(16)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn home_pane(&self) -> Element<'_, Message> {
        if let Some(editor) = &self.editor {
            if let Some(reason) = &editor.crashed {
                center(
                    column![
                        text("The plugin crashed").size(18),
                        text(reason.as_str()).style(theme::error_text),
                        text("The file on disk was not changed.").style(theme::muted),
                        button("Back").on_press(Message::CloseEditor),
                    ]
                    .spacing(10)
                    .align_x(Alignment::Center),
                )
                .into()
            } else {
                let save = button("Save").on_press_maybe((!editor.saving).then_some(Message::Save));
                column![
                    row![
                        text(editor.file_name()).size(16),
                        text(if editor.dirty { "•" } else { "" }).style(theme::heading),
                        space::horizontal(),
                        save,
                        button("Close")
                            .style(button::secondary)
                            .on_press(Message::CloseEditor),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                    scrollable(
                        container(render(&editor.tree, &editor.ui).map(Message::Editor))
                            .padding([0, 16])
                    )
                    .height(Length::Fill),
                ]
                .spacing(12)
                .into()
            }
        } else if let Some(busy) = &self.busy {
            center(text(busy.as_str()).style(theme::muted)).into()
        } else {
            center(text("Select a save on the left to edit it.").style(theme::muted)).into()
        }
    }

    fn status_bar(&self) -> Element<'_, Message> {
        let message = self
            .busy
            .as_deref()
            .filter(|_| self.editor.is_some())
            .or(self.status.as_deref())
            .unwrap_or("");
        container(text(message).size(12))
            .padding([6, 12])
            .width(Length::Fill)
            .height(Length::Fixed(theme::STATUS_HEIGHT))
            .style(theme::status_bar)
            .into()
    }

    fn modal_view<'a>(&'a self, modal: &'a Modal) -> Element<'a, Message> {
        let body: Column<'a, Message> = match modal {
            Modal::ChoosePlugin { path, candidates } => {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let mut col = column![
                    text("Choose a plugin").size(18),
                    text(format!("Several plugins can open {name}.")).style(theme::muted),
                ]
                .spacing(10);
                col = col.extend(candidates.iter().map(|plugin| {
                    button(text(plugin.manifest.name.as_str()))
                        .width(Length::Fill)
                        .on_press(Message::ChoosePlugin {
                            path: path.clone(),
                            plugin_id: plugin.manifest.id.clone(),
                        })
                        .into()
                }));
                col.push(
                    button("Cancel")
                        .style(button::secondary)
                        .on_press(Message::CancelModal),
                )
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
                column![
                    text("Unsaved changes").size(18),
                    text(format!("Save changes to {file} before {action}?")),
                    row![
                        button("Save").on_press(Message::GuardSave),
                        button("Discard")
                            .style(theme::danger_button)
                            .on_press(Message::GuardDiscard),
                        button("Cancel")
                            .style(button::secondary)
                            .on_press(Message::CancelModal),
                    ]
                    .spacing(8),
                ]
                .spacing(12)
            }
        };
        container(body)
            .padding(20)
            .max_width(420)
            .style(theme::modal)
            .into()
    }
}

/// A rail button at the bottom that opens `screen`.
fn nav_button<'a>(
    label: &'a str,
    tip: &'a str,
    screen: Screen,
    active: bool,
) -> Element<'a, Message> {
    let button = button(center(text(label).size(11)))
        .width(56)
        .height(32)
        .padding(0)
        .style(theme::rail_button(active))
        .on_press(Message::Navigate(screen));
    tooltip(button, text(tip), tooltip::Position::Right).into()
}

fn info_bar(info: &str) -> Element<'_, Message> {
    container(
        row![
            text(info).width(Length::Fill),
            button(text("Dismiss").size(12))
                .style(button::secondary)
                .on_press(Message::DismissInfo),
        ]
        .spacing(12)
        .align_y(Alignment::Center),
    )
    .padding([6, 12])
    .width(Length::Fill)
    .style(theme::info_bar)
    .into()
}

/// A plugin's first letter, used when it ships no icon.
fn initial<'a>(name: &str) -> Element<'a, Message> {
    let letter: String = name
        .chars()
        .next()
        .map(|c| c.to_uppercase().collect())
        .unwrap_or_default();
    text(letter).size(18).into()
}
