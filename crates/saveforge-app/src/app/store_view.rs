//! The Store list and plugin detail screens (spec §5.2 Store), styled as the
//! design's plugin browser.

use iced::widget::{
    Row, button, center, column, container, progress_bar, row, scrollable, space, text, text_input,
};
use iced::{Alignment, Element, Length, Padding};
use saveforge_host::HOST_VERSION;
use saveforge_store::{Catalog, CatalogEntry, EntryStatus};

use super::{App, Message, Screen, is_web_link};
use crate::components::{Size, badge, btn, caption, glyph, hline, panel_title};
use crate::fonts;
use crate::icons::{Icon, icon};
use crate::store::{CatalogState, progress_fraction};
use crate::theme::{self, Tone};

/// Width of a plugin card in the browser grid.
const CARD_WIDTH: f32 = 280.0;

impl App {
    pub(super) fn store_list(&self) -> Element<'_, Message> {
        let search = container(
            row![
                icon(Icon::Search, 11.0, theme::TEXT_FAINT),
                text_input("Search plugins...", &self.store.query)
                    .size(12)
                    .padding([6, 4])
                    .style(theme::stepper_input)
                    .on_input(Message::StoreSearch),
            ]
            .spacing(4)
            .padding(Padding::new(0.0).left(10))
            .align_y(Alignment::Center),
        )
        .width(280)
        .style(theme::stepper);
        let header = row![
            panel_title("Plugin Browser"),
            space::horizontal(),
            search,
            btn("Refresh", Some(Icon::Refresh), Size::Normal).on_press_maybe(
                self.catalog_request
                    .is_none()
                    .then_some(Message::RefreshCatalog)
            ),
        ]
        .spacing(10)
        .align_y(Alignment::Center);
        let body: Element<'_, Message> = match self.loaded_catalog() {
            Err(placeholder) => placeholder,
            Ok(catalog) => {
                let entries = catalog.search(&self.store.query);
                if entries.is_empty() {
                    center(
                        text("No plugins match your search.")
                            .size(12)
                            .style(theme::faint),
                    )
                    .into()
                } else {
                    scrollable(
                        Row::with_children(entries.into_iter().map(|e| self.store_card(e)))
                            .spacing(12)
                            .wrap()
                            .vertical_spacing(12),
                    )
                    .style(theme::scroll)
                    .height(Length::Fill)
                    .into()
                }
            }
        };
        column![header, hline(), body].spacing(14).into()
    }

    fn store_card<'a>(&'a self, entry: &'a CatalogEntry) -> Element<'a, Message> {
        let mut tags = Row::new().spacing(6);
        if let Some((label, tone)) = status_tag(&self.entry_status(entry)) {
            tags = tags.push(badge(label, tone));
        }
        tags = tags.extend(
            entry
                .tags
                .iter()
                .map(|t| badge(t.as_str(), Tone::Plain).into()),
        );
        button(
            column![
                row![
                    glyph(&entry.id, &entry.name, 44.0),
                    column![
                        text(entry.name.as_str()).size(19).font(fonts::PIXEL),
                        caption(format!("by {}", entry.author)),
                    ]
                    .spacing(3),
                ]
                .spacing(10),
                text(entry.description.as_str())
                    .size(11)
                    .style(theme::muted),
                tags.wrap().vertical_spacing(6),
                caption(format!("v{}", entry.latest_version)),
            ]
            .spacing(10),
        )
        .width(CARD_WIDTH)
        .padding(14)
        .style(theme::tile(false))
        .on_press(Message::Navigate(Screen::PluginDetail(entry.id.clone())))
        .into()
    }

    pub(super) fn plugin_detail<'a>(&'a self, id: &'a str) -> Element<'a, Message> {
        let back = btn("Back to Store", None, Size::Small)
            .style(theme::btn_ghost)
            .on_press(Message::Navigate(Screen::Store));
        let body: Element<'a, Message> = match self.loaded_catalog() {
            Err(placeholder) => placeholder,
            Ok(catalog) => match catalog.entry(id) {
                None => text(format!("{id} is not in the catalog."))
                    .size(12)
                    .style(theme::faint)
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
        let mut title = row![text(entry.name.as_str()).size(28).font(fonts::PIXEL)]
            .spacing(10)
            .align_y(Alignment::Center);
        if let Some((label, tone)) = status_tag(&status) {
            title = title.push(badge(label, tone));
        }
        let mut col = column![
            row![
                glyph(&entry.id, &entry.name, 56.0),
                column![
                    title,
                    caption(format!("by {}", entry.author)),
                    text(version).size(11).style(theme::muted),
                ]
                .spacing(4),
            ]
            .spacing(14)
            .align_y(Alignment::Center),
            hline(),
            text(entry.description.as_str()).size(12),
        ]
        .spacing(12)
        .max_width(640);
        if !entry.tags.is_empty() {
            col = col.push(
                text(format!("Tags: {}", entry.tags.join(", ")))
                    .size(11)
                    .style(theme::faint),
            );
        }
        let job = self.store.job.as_ref();
        if let Some(job) = job.filter(|job| job.plugin_id == entry.id) {
            col = col.push(match progress_fraction(job.progress) {
                Some(fraction) => Element::from(
                    progress_bar(0.0..=1.0, fraction)
                        .girth(8)
                        .style(theme::progress),
                ),
                None => caption("Working…").into(),
            });
        }
        // One job at a time: every action waits for the running one.
        let idle = job.is_none();
        let action = |label: &'a str, lead: Icon, message: Message| {
            btn(label, Some(lead), Size::Normal).on_press_maybe(idle.then_some(message))
        };
        let id = entry.id.clone();
        let mut actions = Row::new().spacing(8).align_y(Alignment::Center);
        actions = match status {
            EntryStatus::NotInstalled => actions.push(
                action("Install", Icon::Download, Message::InstallPlugin(id))
                    .style(theme::btn_primary),
            ),
            EntryStatus::UpdateAvailable { .. } => actions
                .push(
                    action("Update", Icon::Download, Message::UpdatePlugin(id.clone()))
                        .style(theme::btn_primary),
                )
                .push(
                    action("Remove", Icon::Trash, Message::RemovePlugin(id))
                        .style(theme::btn_danger),
                ),
            EntryStatus::Installed { .. } => actions.push(
                action("Remove", Icon::Trash, Message::RemovePlugin(id)).style(theme::btn_danger),
            ),
            EntryStatus::Incompatible => {
                let needs = match &entry.max_host_version {
                    Some(max) => format!("Needs SaveForge {}–{max}", entry.min_host_version),
                    None => format!("Needs SaveForge {}", entry.min_host_version),
                };
                actions.push(text(needs).size(12).style(theme::muted))
            }
        };
        if let Some(url) = entry.repository_url.as_ref().filter(|url| is_web_link(url)) {
            actions = actions.push(
                btn("Repository", None, Size::Normal)
                    .style(theme::btn_ghost)
                    .on_press(Message::OpenUrl(url.clone())),
            );
        }
        scrollable(col.push(actions)).style(theme::scroll).into()
    }

    /// The catalog, or what to show instead while there is none.
    fn loaded_catalog(&self) -> Result<&Catalog, Element<'_, Message>> {
        match &self.store.catalog {
            CatalogState::Ready(load) => Ok(&load.catalog),
            CatalogState::NotLoaded | CatalogState::Loading => {
                Err(center(text("Loading…").size(12).style(theme::faint)).into())
            }
            CatalogState::Failed(e) => Err(center(
                text(format!("Could not load the catalog: {e}"))
                    .size(12)
                    .style(theme::error_text),
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

fn status_tag(status: &EntryStatus) -> Option<(&'static str, Tone)> {
    match status {
        EntryStatus::NotInstalled => None,
        EntryStatus::Installed { .. } => Some(("Installed", Tone::Green)),
        EntryStatus::UpdateAvailable { .. } => Some(("Update available", Tone::Amber)),
        EntryStatus::Incompatible => Some(("Incompatible", Tone::Red)),
    }
}
