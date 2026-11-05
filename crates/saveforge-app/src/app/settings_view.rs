//! The Settings and About screens (spec §5.2).

use iced::widget::{Column, checkbox, column, container, row, scrollable, space, text, text_input};
use iced::{Alignment, Element, Length, Padding};
use saveforge_host::{HOST_VERSION, WASMTIME_VERSION};
use saveforge_plugin_api::API_VERSION;

use super::{App, Message, REPOSITORY_URL};
use crate::components::{Size, btn, caption, hline, panel_title, section_title};
use crate::fonts;
use crate::icons::Icon;
use crate::settings_form::SettingsField;
use crate::theme;

/// The main components and their licences, shown in About.
const LICENCES: [(&str, &str); 8] = [
    ("iced", "MIT"),
    ("wasmtime", "Apache-2.0 WITH LLVM-exception"),
    ("rustls", "Apache-2.0 OR ISC OR MIT"),
    ("ureq", "MIT OR Apache-2.0"),
    ("zip", "MIT"),
    ("serde / serde_json", "MIT OR Apache-2.0"),
    ("JetBrains Mono (font)", "OFL-1.1"),
    ("VT323 (font)", "OFL-1.1"),
];

const LINKS: [(&str, &str); 3] = [
    ("Repository", REPOSITORY_URL),
    ("iced", "https://iced.rs"),
    ("wasmtime", "https://wasmtime.dev"),
];

/// Width of the label column, as in the editor's field rows.
const LABEL_WIDTH: f32 = 180.0;

pub(super) fn about<'a>() -> Element<'a, Message> {
    let links = row(LINKS.iter().map(|(label, url)| {
        btn(label, None, Size::Small)
            .on_press(Message::OpenUrl((*url).to_owned()))
            .into()
    }))
    .spacing(8);
    let licences = Column::with_children(LICENCES.iter().map(|(name, licence)| {
        column![
            row![
                text(*name).size(12).width(Length::Fixed(LABEL_WIDTH)),
                text(*licence).size(12).style(theme::muted),
            ],
            hline(),
        ]
        .spacing(8)
        .into()
    }))
    .spacing(8);
    scrollable(
        column![
            panel_title("About"),
            hline(),
            text(format!("SaveForge {}", *HOST_VERSION))
                .size(28)
                .font(fonts::PIXEL),
            column![
                caption(format!("wasmtime {WASMTIME_VERSION}")),
                caption(format!("Plugin API {API_VERSION}")),
            ]
            .spacing(4),
            links,
            container(section_title("Open-source components", None))
                .padding(Padding::new(0.0).top(12)),
            licences,
        ]
        .spacing(12)
        .max_width(640),
    )
    .style(theme::scroll)
    .into()
}

impl App {
    pub(super) fn settings_view(&self) -> Element<'_, Message> {
        let form = &self.settings_form;
        let errors = Column::with_children(
            form.errors
                .iter()
                .map(|e| text(e.as_str()).size(11).style(theme::error_text).into()),
        )
        .spacing(4);
        scrollable(
            column![
                panel_title("Settings"),
                hline(),
                container(section_title("Catalog", None)).padding(Padding::new(0.0).top(8)),
                labelled(
                    "Catalog URL",
                    None,
                    text_input("https://…/catalog.json", &form.catalog_url)
                        .size(12)
                        .padding([6, 10])
                        .style(theme::input)
                        .on_input(|v| Message::SettingsEdited(SettingsField::CatalogUrl, v))
                        .into(),
                ),
                labelled(
                    "Updates",
                    None,
                    checkbox(form.check_updates_on_startup)
                        .label("Check for plugin updates on startup")
                        .size(16)
                        .text_size(12)
                        .style(theme::check)
                        .on_toggle(Message::SettingsToggled)
                        .into(),
                ),
                container(section_title("Folders", None)).padding(Padding::new(0.0).top(16)),
                directory(
                    "Plugin directory",
                    Some("Takes effect after restart."),
                    SettingsField::PluginDirectory,
                    &form.plugin_directory
                ),
                directory(
                    "Backup directory",
                    None,
                    SettingsField::BackupDirectory,
                    &form.backup_directory
                ),
                errors,
                row![
                    space::horizontal(),
                    btn("Check now", Some(Icon::Refresh), Size::Normal)
                        .on_press(Message::CheckUpdatesNow),
                    btn("Save", Some(Icon::Save), Size::Normal)
                        .style(theme::btn_primary)
                        .on_press(Message::SaveSettings),
                ]
                .spacing(8)
                .padding(Padding::new(0.0).top(8)),
            ]
            .spacing(8)
            .max_width(720),
        )
        .style(theme::scroll)
        .into()
    }
}

/// A `.field` row: label (and hint) left, input right, hairline below.
fn labelled<'a>(
    label: &'a str,
    hint: Option<&'a str>,
    input: Element<'a, Message>,
) -> Element<'a, Message> {
    let mut left = column![text(label).size(11).style(theme::muted)].spacing(2);
    if let Some(hint) = hint {
        left = left.push(text(hint).size(10).style(theme::faint));
    }
    column![
        row![left.width(Length::Fixed(LABEL_WIDTH)), input]
            .spacing(16)
            .align_y(Alignment::Center),
        hline(),
    ]
    .spacing(10)
    .padding(Padding::new(0.0).top(4))
    .into()
}

fn directory<'a>(
    label: &'a str,
    hint: Option<&'a str>,
    field: SettingsField,
    value: &'a str,
) -> Element<'a, Message> {
    labelled(
        label,
        hint,
        row![
            text_input("", value)
                .size(12)
                .padding([6, 10])
                .style(theme::input)
                .on_input(move |v| Message::SettingsEdited(field, v))
                .width(Length::Fill),
            btn("Browse…", Some(Icon::Folder), Size::Normal)
                .on_press(Message::BrowseSettingsDir(field)),
        ]
        .spacing(6)
        .align_y(Alignment::Center)
        .into(),
    )
}
