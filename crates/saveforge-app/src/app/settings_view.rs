//! The Settings and About screens (spec §5.2).

use iced::widget::{Column, button, checkbox, column, row, scrollable, text, text_input};
use iced::{Alignment, Element, Length};
use saveforge_host::{HOST_VERSION, WASMTIME_VERSION};
use saveforge_plugin_api::API_VERSION;

use super::{App, Message, REPOSITORY_URL};
use crate::settings_form::SettingsField;
use crate::theme;

/// The main components and their licences, shown in About.
const LICENCES: [(&str, &str); 6] = [
    ("iced", "MIT"),
    ("wasmtime", "Apache-2.0 WITH LLVM-exception"),
    ("rustls", "Apache-2.0 OR ISC OR MIT"),
    ("ureq", "MIT OR Apache-2.0"),
    ("zip", "MIT"),
    ("serde / serde_json", "MIT OR Apache-2.0"),
];

const LINKS: [(&str, &str); 3] = [
    ("Repository", REPOSITORY_URL),
    ("iced", "https://iced.rs"),
    ("wasmtime", "https://wasmtime.dev"),
];

pub(super) fn about<'a>() -> Element<'a, Message> {
    let links = row(LINKS.iter().map(|(label, url)| {
        button(text(*label).size(13))
            .style(button::secondary)
            .on_press(Message::OpenUrl((*url).to_owned()))
            .into()
    }))
    .spacing(8);
    let licences = Column::with_children(LICENCES.iter().map(|(name, licence)| {
        row![
            text(*name).width(Length::Fixed(180.0)),
            text(*licence).style(theme::muted),
        ]
        .into()
    }))
    .spacing(4);
    scrollable(
        column![
            text(format!("SaveForge {}", *HOST_VERSION)).size(22),
            text(format!("wasmtime {WASMTIME_VERSION}")).style(theme::muted),
            text(format!("Plugin API {API_VERSION}")).style(theme::muted),
            links,
            text("Open-source components").size(16),
            licences,
        ]
        .spacing(12),
    )
    .into()
}

impl App {
    pub(super) fn settings_view(&self) -> Element<'_, Message> {
        let form = &self.settings_form;
        let errors = Column::with_children(
            form.errors
                .iter()
                .map(|e| text(e.as_str()).style(theme::error_text).into()),
        )
        .spacing(4);
        scrollable(
            column![
                text("Settings").size(22),
                labelled(
                    "Catalog URL",
                    text_input("https://…/catalog.json", &form.catalog_url)
                        .on_input(|v| Message::SettingsEdited(SettingsField::CatalogUrl, v))
                        .into(),
                ),
                directory(
                    "Plugin directory",
                    SettingsField::PluginDirectory,
                    &form.plugin_directory
                ),
                text("Takes effect after restart.")
                    .size(12)
                    .style(theme::muted),
                directory(
                    "Backup directory",
                    SettingsField::BackupDirectory,
                    &form.backup_directory
                ),
                checkbox(form.check_updates_on_startup)
                    .label("Check for plugin updates on startup")
                    .on_toggle(Message::SettingsToggled),
                errors,
                row![
                    button("Save").on_press(Message::SaveSettings),
                    button("Check now")
                        .style(button::secondary)
                        .on_press(Message::CheckUpdatesNow),
                ]
                .spacing(8),
            ]
            .spacing(12)
            .max_width(640),
        )
        .into()
    }
}

fn labelled<'a>(label: &'a str, input: Element<'a, Message>) -> Element<'a, Message> {
    column![text(label).size(13).style(theme::muted), input]
        .spacing(4)
        .into()
}

fn directory<'a>(label: &'a str, field: SettingsField, value: &'a str) -> Element<'a, Message> {
    labelled(
        label,
        row![
            text_input("", value)
                .on_input(move |v| Message::SettingsEdited(field, v))
                .width(Length::Fill),
            button(text("Browse…").size(12)).on_press(Message::BrowseSettingsDir(field)),
        ]
        .spacing(6)
        .align_y(Alignment::Center)
        .into(),
    )
}
