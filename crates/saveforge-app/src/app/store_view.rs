//! The Store list and plugin detail screens (spec §5.2 Store).

use iced::widget::{
    Column, button, center, column, progress_bar, row, scrollable, text, text_input,
};
use iced::{Alignment, Element, Font, Length, font};
use saveforge_host::HOST_VERSION;
use saveforge_store::{Catalog, CatalogEntry, EntryStatus};

use super::{App, Message, Screen, is_web_link};
use crate::store::{CatalogState, progress_fraction};
use crate::theme;

const BOLD: Font = Font {
    weight: font::Weight::Bold,
    ..Font::DEFAULT
};

impl App {
    pub(super) fn store_list(&self) -> Element<'_, Message> {
        let header = row![
            text_input("Search plugins", &self.store.query)
                .on_input(Message::StoreSearch)
                .width(Length::Fill),
            button("Refresh").style(button::secondary).on_press_maybe(
                self.catalog_request
                    .is_none()
                    .then_some(Message::RefreshCatalog)
            ),
        ]
        .spacing(8)
        .align_y(Alignment::Center);
        let body: Element<'_, Message> = match self.loaded_catalog() {
            Err(placeholder) => placeholder,
            Ok(catalog) => {
                let entries = catalog.search(&self.store.query);
                if entries.is_empty() {
                    center(text("No plugins match your search.").style(theme::muted)).into()
                } else {
                    scrollable(
                        Column::with_children(entries.into_iter().map(|e| self.store_row(e)))
                            .spacing(8),
                    )
                    .height(Length::Fill)
                    .into()
                }
            }
        };
        column![text("Store").size(22), header, body]
            .spacing(12)
            .into()
    }

    fn store_row<'a>(&'a self, entry: &'a CatalogEntry) -> Element<'a, Message> {
        let mut title = row![text(entry.name.as_str()).font(BOLD)]
            .spacing(8)
            .align_y(Alignment::Center);
        if let Some(tag) = status_tag(&self.entry_status(entry)) {
            title = title.push(text(tag).size(12).style(theme::heading));
        }
        button(
            column![
                title,
                text(format!("by {}", entry.author))
                    .size(12)
                    .style(theme::muted),
                text(entry.description.as_str()).size(13),
            ]
            .spacing(4),
        )
        .width(Length::Fill)
        .padding(10)
        .style(theme::save_row(false))
        .on_press(Message::Navigate(Screen::PluginDetail(entry.id.clone())))
        .into()
    }

    pub(super) fn plugin_detail<'a>(&'a self, id: &'a str) -> Element<'a, Message> {
        let back = button(text("Back to Store").size(12))
            .style(button::secondary)
            .on_press(Message::Navigate(Screen::Store));
        let body: Element<'a, Message> = match self.loaded_catalog() {
            Err(placeholder) => placeholder,
            Ok(catalog) => match catalog.entry(id) {
                None => text(format!("{id} is not in the catalog."))
                    .style(theme::muted)
                    .into(),
                Some(entry) => self.detail_body(entry),
            },
        };
        column![back, body].spacing(12).into()
    }

    fn detail_body<'a>(&'a self, entry: &'a CatalogEntry) -> Element<'a, Message> {
        let status = self.entry_status(entry);
        let version = match &status {
            EntryStatus::Installed { version }
            | EntryStatus::UpdateAvailable { installed: version } => {
                format!("Version {} · installed {version}", entry.latest_version)
            }
            EntryStatus::NotInstalled | EntryStatus::Incompatible => {
                format!("Version {}", entry.latest_version)
            }
        };
        let mut col = column![
            text(entry.name.as_str()).size(22),
            text(format!("by {}", entry.author)).style(theme::muted),
            text(version),
            text(entry.description.as_str()),
        ]
        .spacing(10);
        if !entry.tags.is_empty() {
            col = col.push(
                text(format!("Tags: {}", entry.tags.join(", ")))
                    .size(12)
                    .style(theme::muted),
            );
        }
        if let Some(url) = entry.repository_url.as_ref().filter(|url| is_web_link(url)) {
            col = col.push(
                button(text("Repository").size(12))
                    .style(button::secondary)
                    .on_press(Message::OpenUrl(url.clone())),
            );
        }
        let job = self.store.job.as_ref();
        if let Some(job) = job.filter(|job| job.plugin_id == entry.id) {
            col = col.push(match progress_fraction(job.progress) {
                Some(fraction) => Element::from(progress_bar(0.0..=1.0, fraction)),
                None => text("Working…").style(theme::muted).into(),
            });
        }
        // One job at a time: every action waits for the running one.
        let idle = job.is_none();
        let action = |label: &'a str, message: Message| {
            button(label).on_press_maybe(idle.then_some(message))
        };
        let id = entry.id.clone();
        let actions: Element<'a, Message> = match status {
            EntryStatus::NotInstalled => action("Install", Message::InstallPlugin(id)).into(),
            EntryStatus::UpdateAvailable { .. } => row![
                action("Update", Message::UpdatePlugin(id.clone())),
                action("Remove", Message::RemovePlugin(id)).style(theme::danger_button),
            ]
            .spacing(8)
            .into(),
            EntryStatus::Installed { .. } => action("Remove", Message::RemovePlugin(id))
                .style(theme::danger_button)
                .into(),
            EntryStatus::Incompatible => {
                let needs = match &entry.max_host_version {
                    Some(max) => format!("Needs SaveForge {}–{max}", entry.min_host_version),
                    None => format!("Needs SaveForge {}", entry.min_host_version),
                };
                text(needs).style(theme::muted).into()
            }
        };
        scrollable(col.push(actions)).into()
    }

    /// The catalog, or what to show instead while there is none.
    fn loaded_catalog(&self) -> Result<&Catalog, Element<'_, Message>> {
        match &self.store.catalog {
            CatalogState::Ready(load) => Ok(&load.catalog),
            CatalogState::NotLoaded | CatalogState::Loading => {
                Err(center(text("Loading…").style(theme::muted)).into())
            }
            CatalogState::Failed(e) => Err(center(
                text(format!("Could not load the catalog: {e}")).style(theme::error_text),
            )
            .into()),
        }
    }

    fn entry_status(&self, entry: &CatalogEntry) -> EntryStatus {
        let installed = self
            .registry()
            .and_then(|registry| registry.get(&entry.id))
            .and_then(saveforge_host::PluginEntry::version);
        entry.status(installed.as_ref(), &HOST_VERSION)
    }
}

fn status_tag(status: &EntryStatus) -> Option<&'static str> {
    match status {
        EntryStatus::NotInstalled => None,
        EntryStatus::Installed { .. } => Some("Installed"),
        EntryStatus::UpdateAvailable { .. } => Some("Update available"),
        EntryStatus::Incompatible => Some("Incompatible"),
    }
}
