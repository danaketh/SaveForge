//! Small building blocks shared by the screens: buttons, badges, plugin
//! glyphs, section headings and dividers in the `SaveEditor` design's style.

use iced::widget::{Button, Container, button, center, column, container, row, space, text};
use iced::{Alignment, Background, Color, Degrees, Element, Gradient, Length, gradient};

use crate::fonts;
use crate::icons::{Icon, icon};
use crate::theme::{self, Tone};

/// Design colours a plugin glyph is painted with, picked by plugin id.
const GLYPH_COLORS: [Color; 9] = [
    Color::from_rgb8(0x8F, 0xD6, 0x8F),
    Color::from_rgb8(0xD6, 0xA0, 0x4A),
    Color::from_rgb8(0x6F, 0xB4, 0xFF),
    Color::from_rgb8(0x9A, 0x7F, 0xE0),
    Color::from_rgb8(0xFF, 0x9A, 0xCC),
    Color::from_rgb8(0xE5, 0x48, 0x4D),
    Color::from_rgb8(0x6F, 0xD4, 0x79),
    Color::from_rgb8(0xFF, 0xD4, 0x79),
    Color::from_rgb8(0xC9, 0x7A, 0x4A),
];

/// Button sizes (`.btn` and `.btn.sm`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Size {
    Normal,
    Small,
}

/// A labelled button with an optional leading icon, in the default style.
pub fn btn<'a, M: 'a>(label: &'a str, lead: Option<Icon>, size: Size) -> Button<'a, M> {
    let (text_size, padding, icon_size) = match size {
        Size::Normal => (12.0, [6, 12], 12.0),
        Size::Small => (11.0, [3, 8], 11.0),
    };
    let label = text(label).size(text_size).font(fonts::MONO_MEDIUM);
    let content: Element<'a, M> = match lead {
        Some(lead) => row![icon(lead, icon_size, theme::TEXT), label]
            .spacing(6)
            .align_y(Alignment::Center)
            .into(),
        None => label.into(),
    };
    button(content).padding(padding).style(theme::btn)
}

/// A square icon-only button (`.icn`).
pub fn icon_btn<'a, M: 'a>(lead: Icon, size: f32) -> Button<'a, M> {
    button(center(icon(lead, 12.0, theme::TEXT_DIM)))
        .width(size)
        .height(size)
        .padding(0)
        .style(theme::icon_button)
}

/// A small uppercase tag (`.badge`).
pub fn badge<'a, M: 'a>(label: impl text::IntoFragment<'a>, tone: Tone) -> Container<'a, M> {
    container(text(label).size(10).font(fonts::MONO_MEDIUM))
        .padding([1, 6])
        .style(theme::badge(tone))
}

/// A plugin's square monogram, coloured by its id (`PluginGlyph`).
pub fn glyph<'a, M: 'a>(id: &str, name: &str, size: f32) -> Element<'a, M> {
    let base = glyph_color(id);
    let fill = Gradient::Linear(
        gradient::Linear::new(Degrees(135.0))
            .add_stop(0.0, base)
            .add_stop(1.0, shade(base, -0.25)),
    );
    container(
        text(monogram(name))
            .size(size * 0.52)
            .font(fonts::PIXEL)
            .color(theme::ON_GLYPH),
    )
    .center(Length::Fixed(size))
    .style(move |_| container::Style {
        background: Some(Background::Gradient(fill)),
        border: iced::Border {
            color: Color::from_rgba8(0, 0, 0, 0.3),
            width: 1.0,
            radius: 0.0.into(),
        },
        ..container::Style::default()
    })
    .into()
}

fn glyph_color(id: &str) -> Color {
    // FNV-1a: stable across runs, unlike the std hasher.
    let hash = id.bytes().fold(0x811c_9dc5_u32, |h, b| {
        (h ^ u32::from(b)).wrapping_mul(0x0100_0193)
    });
    GLYPH_COLORS[hash as usize % GLYPH_COLORS.len()]
}

/// `color` lightened (`amount > 0`) or darkened by a fraction of full scale.
fn shade(color: Color, amount: f32) -> Color {
    let adjust = |c: f32| (c + amount).clamp(0.0, 1.0);
    Color {
        r: adjust(color.r),
        g: adjust(color.g),
        b: adjust(color.b),
        ..color
    }
}

/// Up to two letters: the initials of the first two words, else the first
/// two letters of the name.
pub fn monogram(name: &str) -> String {
    let words: Vec<Vec<char>> = name
        .split(|c: char| c.is_whitespace() || c == '-' || c == '_')
        .map(|w| {
            w.chars()
                .filter(|c| c.is_alphanumeric())
                .collect::<Vec<_>>()
        })
        .filter(|w| !w.is_empty())
        .collect();
    let letters: String = match words.as_slice() {
        [] => String::new(),
        [one] => one.iter().take(2).collect(),
        [first, second, ..] => [first[0], second[0]].iter().collect(),
    };
    letters.to_uppercase()
}

/// An amber pixel heading over a full-width line (`Section`).
pub fn section_title<'a, M: 'a>(title: &'a str, aside: Option<Element<'a, M>>) -> Element<'a, M> {
    let mut head = row![
        text(title)
            .size(18)
            .font(fonts::PIXEL)
            .style(theme::heading)
            .width(Length::Fill)
    ]
    .align_y(Alignment::End);
    if let Some(aside) = aside {
        head = head.push(aside);
    }
    column![head, hline()].spacing(6).into()
}

/// A pixel-face title in amber caps, as used for panel headings.
pub fn panel_title<'a, M: 'a>(title: &str) -> Element<'a, M> {
    text(title.to_uppercase())
        .size(18)
        .font(fonts::PIXEL)
        .style(theme::heading)
        .into()
}

/// A 1px horizontal line.
pub fn hline<'a, M: 'a>() -> Element<'a, M> {
    container(space())
        .width(Length::Fill)
        .height(1)
        .style(theme::line)
        .into()
}

/// A 1px vertical line between panes.
pub fn vline<'a, M: 'a>() -> Element<'a, M> {
    container(space())
        .width(1)
        .height(Length::Fill)
        .style(theme::line)
        .into()
}

/// A small square indicator (unsaved marker, status LED).
pub fn dot<'a, M: 'a>(color: Color, size: f32) -> Element<'a, M> {
    container(space())
        .width(size)
        .height(size)
        .style(theme::dot(color))
        .into()
}

/// Faint uppercase caption, such as `SAVES · 3`.
pub fn caption<'a>(label: impl text::IntoFragment<'a>) -> text::Text<'a> {
    text(label).size(10).style(theme::faint)
}
