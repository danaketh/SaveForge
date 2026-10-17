//! The Settings and About screens (spec §5.2).

use iced::widget::{Column, button, column, row, scrollable, text};
use iced::{Element, Length};
use saveforge_host::{HOST_VERSION, WASMTIME_VERSION};
use saveforge_plugin_api::API_VERSION;

use super::{Message, REPOSITORY_URL};
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
