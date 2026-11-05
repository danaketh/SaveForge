//! The design's small stroke icons (`primitives.jsx`), drawn as SVG and
//! tinted with the caller's colour.

use iced::widget::{Svg, svg};
use iced::{Color, Length};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Search,
    Plus,
    Close,
    Check,
    Save,
    Folder,
    File,
    Cog,
    Info,
    Download,
    Refresh,
    Trash,
    Warn,
    Pencil,
}

impl Icon {
    /// The SVG body, on a 16×16 grid with square strokes.
    fn paths(self) -> &'static str {
        match self {
            Self::Search => r#"<circle cx="7" cy="7" r="4.5"/><path d="M10.5 10.5L14 14"/>"#,
            Self::Plus => r#"<path d="M8 3v10M3 8h10"/>"#,
            Self::Close => r#"<path d="M3 3l10 10M13 3L3 13"/>"#,
            Self::Check => r#"<path d="M3 8l3.5 3.5L13 4"/>"#,
            Self::Save => {
                r#"<path d="M2.5 2.5h8L13.5 5.5v8h-11v-11zM5 2.5v4h5v-4M4 8.5h8v5h-8z"/>"#
            }
            Self::Folder => r#"<path d="M2 4h4l2 2h6v8H2z"/>"#,
            Self::File => r#"<path d="M3 2h7l3 3v9H3zM10 2v3h3"/>"#,
            Self::Cog => {
                r#"<circle cx="8" cy="8" r="2.5"/><path d="M8 1v2M8 13v2M1 8h2M13 8h2M3 3l1.5 1.5M11.5 11.5L13 13M13 3l-1.5 1.5M4.5 11.5L3 13"/>"#
            }
            Self::Info => r#"<circle cx="8" cy="8" r="6"/><path d="M8 7v4.5M8 4.5v.5"/>"#,
            Self::Download => r#"<path d="M8 2v8M4 7l4 4 4-4M2 14h12"/>"#,
            Self::Refresh => r#"<path d="M14 3v4h-4M2 13V9h4M13 7a5 5 0 00-9-2M3 9a5 5 0 009 2"/>"#,
            Self::Trash => r#"<path d="M3 4h10M5 4V2h6v2M4 4l1 10h6l1-10M7 7v5M9 7v5"/>"#,
            Self::Warn => r#"<path d="M8 2L1 14h14zM8 6v4M8 12v.5"/>"#,
            Self::Pencil => r#"<path d="M11 2l3 3-8 8H3v-3zM9.5 3.5l3 3"/>"#,
        }
    }

    fn handle(self) -> svg::Handle {
        let source = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="black" stroke-width="1.5" stroke-linecap="square" stroke-linejoin="miter">{}</svg>"#,
            self.paths()
        );
        svg::Handle::from_memory(source.into_bytes())
    }
}

/// `icon` at `size` px, tinted `color`.
pub fn icon<'a>(icon: Icon, size: f32, color: Color) -> Svg<'a> {
    svg(icon.handle())
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .style(move |_, _| svg::Style { color: Some(color) })
}
