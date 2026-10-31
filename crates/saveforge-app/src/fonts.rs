//! The bundled fonts (SIL Open Font License 1.1; the licence texts are next
//! to the files in `assets/fonts/`).
//!
//! `JetBrains Mono` is the default text face; `VT323` is the pixel face for
//! headings and names, as in the design.

use std::borrow::Cow;

use iced::Font;
use iced::font::{Family, Weight};

/// `JetBrains Mono`, the default face.
pub const MONO: Font = Font::with_name("JetBrains Mono");

pub const MONO_MEDIUM: Font = Font {
    weight: Weight::Medium,
    ..MONO
};

pub const MONO_BOLD: Font = Font {
    weight: Weight::Bold,
    ..MONO
};

/// `VT323`, the pixel heading face.
pub const PIXEL: Font = Font {
    family: Family::Name("VT323"),
    ..Font::DEFAULT
};

/// The font files to register with iced at startup.
pub fn files() -> Vec<Cow<'static, [u8]>> {
    vec![
        Cow::Borrowed(include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf").as_slice()),
        Cow::Borrowed(include_bytes!("../assets/fonts/JetBrainsMono-Medium.ttf").as_slice()),
        Cow::Borrowed(include_bytes!("../assets/fonts/JetBrainsMono-Bold.ttf").as_slice()),
        Cow::Borrowed(include_bytes!("../assets/fonts/VT323-Regular.ttf").as_slice()),
    ]
}
