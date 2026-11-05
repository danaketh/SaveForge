//! The `SaveEditor` design's look: a CRT-dark palette with amber phosphor accents,
//! square corners and pixel headings (design `styles.css`, spec §5.1).
//!
//! Widgets take their style from the functions here rather than from the
//! theme palette, so every corner stays square and every colour comes from
//! the constants below.

use iced::border::Radius;
use iced::widget::{
    button, checkbox, container, overlay::menu, pick_list, progress_bar, rule, scrollable, slider,
    text, text_input,
};
use iced::{Background, Border, Color, Shadow, Theme, Vector};

pub const BG0: Color = Color::from_rgb8(0x0B, 0x0D, 0x14);
pub const BG1: Color = Color::from_rgb8(0x12, 0x15, 0x1F);
pub const BG2: Color = Color::from_rgb8(0x1A, 0x1E, 0x2B);
pub const BG3: Color = Color::from_rgb8(0x23, 0x28, 0x38);
pub const BG4: Color = Color::from_rgb8(0x2D, 0x33, 0x46);
pub const LINE: Color = Color::from_rgb8(0x2A, 0x30, 0x44);
pub const LINE2: Color = Color::from_rgb8(0x3A, 0x41, 0x59);
/// Border of a hovered button.
const LINE_HOVER: Color = Color::from_rgb8(0x4A, 0x51, 0x6A);
pub const TEXT: Color = Color::from_rgb8(0xE6, 0xE9, 0xF2);
pub const TEXT_DIM: Color = Color::from_rgb8(0x9A, 0xA2, 0xB8);
pub const TEXT_FAINT: Color = Color::from_rgb8(0x5B, 0x62, 0x76);
pub const AMBER: Color = Color::from_rgb8(0xF5, 0xB9, 0x42);
pub const AMBER_DIM: Color = Color::from_rgb8(0xC6, 0x8F, 0x2B);
const AMBER_HOVER: Color = Color::from_rgb8(0xFF, 0xC9, 0x54);
pub const AMBER_GLOW: Color = Color::from_rgba8(0xF5, 0xB9, 0x42, 0.22);
/// Text on an amber background.
pub const ON_AMBER: Color = Color::from_rgb8(0x1A, 0x10, 0x00);
pub const GREEN: Color = Color::from_rgb8(0x6F, 0xD4, 0x79);
const GREEN_LINE: Color = Color::from_rgb8(0x3A, 0x6B, 0x40);
pub const RED: Color = Color::from_rgb8(0xE5, 0x48, 0x4D);
const RED_BG: Color = Color::from_rgb8(0x3A, 0x14, 0x14);
const RED_BG_HOVER: Color = Color::from_rgb8(0x4A, 0x1A, 0x1A);
pub const RED_LINE: Color = Color::from_rgb8(0x6B, 0x1C, 0x1F);
pub const RED_TEXT: Color = Color::from_rgb8(0xFF, 0x8B, 0x8E);
pub const BLUE: Color = Color::from_rgb8(0x6F, 0xB4, 0xFF);
/// Glyph text on a plugin monogram.
pub const ON_GLYPH: Color = BG0;

pub const RAIL_WIDTH: f32 = 68.0;
pub const PANEL_WIDTH: f32 = 280.0;
pub const STATUS_HEIGHT: f32 = 22.0;
pub const TOOLBAR_HEIGHT: f32 = 40.0;

pub fn theme() -> Theme {
    Theme::custom(
        "SaveForge Dark".to_owned(),
        iced::theme::Palette {
            background: BG1,
            text: TEXT,
            primary: AMBER,
            success: GREEN,
            warning: AMBER,
            danger: RED,
        },
    )
}

fn square(color: Color, width: f32) -> Border {
    Border {
        color,
        width,
        radius: Radius::from(0.0),
    }
}

fn none() -> Border {
    square(Color::TRANSPARENT, 0.0)
}

/// `color` at 40% opacity, as the design draws disabled controls.
fn faded(color: Color) -> Color {
    Color {
        a: color.a * 0.4,
        ..color
    }
}

fn filled(background: Color) -> container::Style {
    container::Style {
        background: Some(Background::Color(background)),
        text_color: Some(TEXT),
        ..container::Style::default()
    }
}

fn boxed(background: Color, line: Color) -> container::Style {
    container::Style {
        border: square(line, 1.0),
        ..filled(background)
    }
}

// ───────────── containers ─────────────

pub fn bg0(_: &Theme) -> container::Style {
    filled(BG0)
}

pub fn bg1(_: &Theme) -> container::Style {
    filled(BG1)
}

pub fn bg2(_: &Theme) -> container::Style {
    filled(BG2)
}

/// A 1px divider between panes (iced borders cannot be one-sided).
pub fn line(_: &Theme) -> container::Style {
    filled(LINE)
}

/// `.panel`: a raised card.
pub fn panel(_: &Theme) -> container::Style {
    boxed(BG2, LINE)
}

/// The inset well behind code-like content.
pub fn well(_: &Theme) -> container::Style {
    boxed(BG0, LINE)
}

pub fn status_bar(_: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TEXT_FAINT),
        ..filled(BG1)
    }
}

/// `.savebar`: the amber strip for unsaved changes and notices.
pub fn notice_bar(_: &Theme) -> container::Style {
    container::Style {
        text_color: Some(AMBER),
        border: square(AMBER_DIM, 1.0),
        ..filled(Color::from_rgba8(0xF5, 0xB9, 0x42, 0.08))
    }
}

pub fn modal(_: &Theme) -> container::Style {
    container::Style {
        shadow: drop_shadow(),
        ..boxed(BG2, LINE2)
    }
}

pub fn backdrop(_: &Theme) -> container::Style {
    filled(Color::from_rgba8(0, 0, 0, 0.6))
}

pub fn tooltip(_: &Theme) -> container::Style {
    container::Style {
        shadow: drop_shadow(),
        ..boxed(BG2, LINE2)
    }
}

/// The toast's body; [`toast_edge`] is its coloured left edge.
pub fn toast(_: &Theme) -> container::Style {
    container::Style {
        shadow: drop_shadow(),
        ..boxed(BG2, LINE2)
    }
}

pub fn toast_edge(color: Color) -> impl Fn(&Theme) -> container::Style {
    move |_| filled(color)
}

/// The amber marker beside the selected rail glyph.
pub fn marker(_: &Theme) -> container::Style {
    filled(AMBER)
}

/// A small square indicator, such as the unsaved dot.
pub fn dot(color: Color) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        shadow: Shadow {
            color: Color { a: 0.5, ..color },
            offset: Vector::ZERO,
            blur_radius: 6.0,
        },
        ..filled(color)
    }
}

/// The danger-zone and crash frame.
pub fn danger_box(_: &Theme) -> container::Style {
    boxed(Color::from_rgba8(0x6B, 0x1C, 0x1F, 0.1), RED_LINE)
}

/// Badge tones (`.badge`, `.badge.amber`, …).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Plain,
    Amber,
    Green,
    Red,
}

pub fn badge(tone: Tone) -> impl Fn(&Theme) -> container::Style {
    move |_| {
        let (background, line, color) = match tone {
            Tone::Plain => (BG3, LINE, TEXT_DIM),
            Tone::Amber => (Color::from_rgba8(0xF5, 0xB9, 0x42, 0.12), AMBER_DIM, AMBER),
            Tone::Green => (Color::from_rgba8(0x6F, 0xD4, 0x79, 0.10), GREEN_LINE, GREEN),
            Tone::Red => (
                Color::from_rgba8(0xE5, 0x48, 0x4D, 0.10),
                RED_LINE,
                RED_TEXT,
            ),
        };
        container::Style {
            text_color: Some(color),
            ..boxed(background, line)
        }
    }
}

/// The slot-number chip on a save card.
pub fn chip(_: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TEXT_FAINT),
        ..boxed(BG1, LINE)
    }
}

fn drop_shadow() -> Shadow {
    Shadow {
        color: Color::from_rgba8(0, 0, 0, 0.5),
        offset: Vector::new(0.0, 8.0),
        blur_radius: 24.0,
    }
}

// ───────────── buttons ─────────────

fn button_style(background: Color, text_color: Color, line: Color) -> button::Style {
    button::Style {
        background: Some(Background::Color(background)),
        text_color,
        border: square(line, 1.0),
        ..button::Style::default()
    }
}

/// `.btn`: the default raised button.
pub fn btn(_: &Theme, status: button::Status) -> button::Style {
    match status {
        button::Status::Active => button_style(BG3, TEXT, LINE2),
        button::Status::Hovered | button::Status::Pressed => button_style(BG4, TEXT, LINE_HOVER),
        button::Status::Disabled => button_style(faded(BG3), faded(TEXT), faded(LINE2)),
    }
}

/// `.btn.primary`: amber, for the one main action.
pub fn btn_primary(_: &Theme, status: button::Status) -> button::Style {
    match status {
        button::Status::Active => button_style(AMBER, ON_AMBER, AMBER_DIM),
        button::Status::Hovered | button::Status::Pressed => {
            button_style(AMBER_HOVER, ON_AMBER, AMBER_DIM)
        }
        button::Status::Disabled => button_style(faded(AMBER), faded(ON_AMBER), faded(AMBER_DIM)),
    }
}

/// `.btn.danger`: destructive actions (Remove, Discard).
pub fn btn_danger(_: &Theme, status: button::Status) -> button::Style {
    match status {
        button::Status::Active => button_style(RED_BG, RED_TEXT, RED_LINE),
        button::Status::Hovered | button::Status::Pressed => {
            button_style(RED_BG_HOVER, RED_TEXT, RED_LINE)
        }
        button::Status::Disabled => button_style(faded(RED_BG), faded(RED_TEXT), faded(RED_LINE)),
    }
}

/// `.btn.ghost`: no chrome until hovered.
pub fn btn_ghost(_: &Theme, status: button::Status) -> button::Style {
    match status {
        button::Status::Active => button_style(Color::TRANSPARENT, TEXT_DIM, Color::TRANSPARENT),
        button::Status::Hovered | button::Status::Pressed => {
            button_style(BG3, TEXT, Color::TRANSPARENT)
        }
        button::Status::Disabled => {
            button_style(Color::TRANSPARENT, TEXT_FAINT, Color::TRANSPARENT)
        }
    }
}

/// `.icn`: a small icon-only button.
pub fn icon_button(_: &Theme, status: button::Status) -> button::Style {
    match status {
        button::Status::Active => button_style(Color::TRANSPARENT, TEXT_DIM, Color::TRANSPARENT),
        button::Status::Hovered | button::Status::Pressed => button_style(BG3, TEXT, LINE),
        button::Status::Disabled => {
            button_style(Color::TRANSPARENT, TEXT_FAINT, Color::TRANSPARENT)
        }
    }
}

/// `.tile`: a card that can be selected (saves, store entries).
pub fn tile(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        if active {
            return button::Style {
                shadow: Shadow {
                    color: AMBER_GLOW,
                    offset: Vector::ZERO,
                    blur_radius: 4.0,
                },
                ..button_style(BG3, TEXT, AMBER)
            };
        }
        match status {
            button::Status::Hovered | button::Status::Pressed => button_style(BG3, TEXT, LINE2),
            button::Status::Active | button::Status::Disabled => button_style(BG2, TEXT, LINE),
        }
    }
}

/// A foldable group header in the save list.
pub fn group_header(_: &Theme, status: button::Status) -> button::Style {
    match status {
        button::Status::Hovered | button::Status::Pressed => {
            button_style(BG2, TEXT, Color::TRANSPARENT)
        }
        button::Status::Active | button::Status::Disabled => {
            button_style(Color::TRANSPARENT, TEXT_DIM, Color::TRANSPARENT)
        }
    }
}

/// `.tab`: a tab with an amber underline when active (drawn by the caller).
pub fn tab(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let (background, color) = match (active, status) {
            (true, _) => (BG2, AMBER),
            (false, button::Status::Hovered | button::Status::Pressed) => (BG2, TEXT),
            (false, _) => (Color::TRANSPARENT, TEXT_DIM),
        };
        button_style(background, color, Color::TRANSPARENT)
    }
}

/// A plugin glyph on the rail; the glyph draws its own face.
pub fn bare(_: &Theme, _: button::Status) -> button::Style {
    button_style(Color::TRANSPARENT, TEXT, Color::TRANSPARENT)
}

/// The rail's "browse plugins" and settings buttons.
pub fn rail_tool(active: bool, framed: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        let background = if active || hovered {
            BG3
        } else {
            Color::TRANSPARENT
        };
        let color = match (active, hovered) {
            (true, _) => AMBER,
            (false, true) => TEXT,
            (false, false) => TEXT_DIM,
        };
        let line = if framed { LINE2 } else { Color::TRANSPARENT };
        button_style(background, color, line)
    }
}

/// `.win-btn`-like stepper ends.
pub fn stepper_end(_: &Theme, status: button::Status) -> button::Style {
    match status {
        button::Status::Active => button_style(BG3, TEXT, Color::TRANSPARENT),
        button::Status::Hovered | button::Status::Pressed => {
            button_style(BG4, TEXT, Color::TRANSPARENT)
        }
        button::Status::Disabled => button_style(BG3, TEXT_FAINT, Color::TRANSPARENT),
    }
}

// ───────────── inputs ─────────────

pub fn input(_: &Theme, status: text_input::Status) -> text_input::Style {
    let line = match status {
        text_input::Status::Focused { .. } => AMBER,
        text_input::Status::Hovered => LINE_HOVER,
        text_input::Status::Active | text_input::Status::Disabled => LINE2,
    };
    text_input::Style {
        background: Background::Color(BG1),
        border: square(line, 1.0),
        icon: TEXT_FAINT,
        placeholder: TEXT_FAINT,
        value: if status == text_input::Status::Disabled {
            TEXT_DIM
        } else {
            TEXT
        },
        selection: Color::from_rgba8(0xF5, 0xB9, 0x42, 0.35),
    }
}

/// The number box between the stepper ends: no frame of its own.
pub fn stepper_input(theme: &Theme, status: text_input::Status) -> text_input::Style {
    text_input::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: none(),
        ..input(theme, status)
    }
}

/// The stepper's frame.
pub fn stepper(_: &Theme) -> container::Style {
    boxed(BG1, LINE2)
}

pub fn pick(_: &Theme, status: pick_list::Status) -> pick_list::Style {
    let line = match status {
        pick_list::Status::Opened { .. } => AMBER,
        pick_list::Status::Hovered => LINE_HOVER,
        pick_list::Status::Active => LINE2,
    };
    pick_list::Style {
        text_color: TEXT,
        placeholder_color: TEXT_FAINT,
        handle_color: TEXT_DIM,
        background: Background::Color(BG1),
        border: square(line, 1.0),
    }
}

pub fn pick_menu(_: &Theme) -> menu::Style {
    menu::Style {
        background: Background::Color(BG2),
        border: square(LINE2, 1.0),
        text_color: TEXT,
        selected_text_color: AMBER,
        selected_background: Background::Color(BG3),
        shadow: drop_shadow(),
    }
}

/// `.check`: square, amber when ticked.
pub fn check(_: &Theme, status: checkbox::Status) -> checkbox::Style {
    let (checked, hovered, disabled) = match status {
        checkbox::Status::Active { is_checked } => (is_checked, false, false),
        checkbox::Status::Hovered { is_checked } => (is_checked, true, false),
        checkbox::Status::Disabled { is_checked } => (is_checked, false, true),
    };
    let (background, line) = match (checked, hovered) {
        (true, _) => (AMBER, AMBER_DIM),
        (false, true) => (BG1, LINE_HOVER),
        (false, false) => (BG1, LINE2),
    };
    let fade = |c: Color| if disabled { faded(c) } else { c };
    checkbox::Style {
        background: Background::Color(fade(background)),
        icon_color: fade(ON_AMBER),
        border: square(fade(line), 1.0),
        text_color: Some(fade(TEXT)),
    }
}

pub fn slider_style(_: &Theme, status: slider::Status) -> slider::Style {
    let handle = if status == slider::Status::Dragged {
        AMBER_HOVER
    } else {
        AMBER
    };
    slider::Style {
        rail: slider::Rail {
            backgrounds: (Background::Color(AMBER_DIM), Background::Color(BG1)),
            width: 6.0,
            border: square(LINE2, 1.0),
        },
        handle: slider::Handle {
            shape: slider::HandleShape::Rectangle {
                width: 10,
                border_radius: Radius::from(0.0),
            },
            background: Background::Color(handle),
            border_width: 1.0,
            border_color: AMBER_DIM,
        },
    }
}

pub fn progress(_: &Theme) -> progress_bar::Style {
    progress_bar::Style {
        background: Background::Color(BG1),
        bar: Background::Color(AMBER),
        border: square(LINE2, 1.0),
    }
}

pub fn scroll(_: &Theme, status: scrollable::Status) -> scrollable::Style {
    let hovered = matches!(
        status,
        scrollable::Status::Hovered {
            is_vertical_scrollbar_hovered: true,
            ..
        } | scrollable::Status::Dragged {
            is_vertical_scrollbar_dragged: true,
            ..
        }
    );
    let rail = scrollable::Rail {
        background: None,
        border: none(),
        scroller: scrollable::Scroller {
            background: Background::Color(if hovered { BG4 } else { BG3 }),
            border: none(),
        },
    };
    scrollable::Style {
        container: container::Style::default(),
        vertical_rail: rail,
        horizontal_rail: rail,
        gap: None,
        auto_scroll: scrollable::AutoScroll {
            background: Background::Color(BG2),
            border: square(LINE2, 1.0),
            shadow: Shadow::default(),
            icon: TEXT_DIM,
        },
    }
}

pub fn divider(_: &Theme) -> rule::Style {
    rule::Style {
        color: LINE,
        radius: Radius::from(0.0),
        fill_mode: rule::FillMode::Full,
        snap: true,
    }
}

// ───────────── text ─────────────

pub fn muted(_: &Theme) -> text::Style {
    text::Style {
        color: Some(TEXT_DIM),
    }
}

pub fn faint(_: &Theme) -> text::Style {
    text::Style {
        color: Some(TEXT_FAINT),
    }
}

pub fn heading(_: &Theme) -> text::Style {
    text::Style { color: Some(AMBER) }
}

pub fn success(_: &Theme) -> text::Style {
    text::Style { color: Some(GREEN) }
}

pub fn error_text(_: &Theme) -> text::Style {
    text::Style {
        color: Some(RED_TEXT),
    }
}
