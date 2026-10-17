//! Dark amber theme ported from the .NET app's `Themes/Colors.axaml` (spec §5.1).

use iced::widget::{button, container, text};
use iced::{Background, Border, Color, Theme};

pub const BG0: Color = Color::from_rgb8(0x0B, 0x0D, 0x14);
pub const BG1: Color = Color::from_rgb8(0x12, 0x15, 0x1F);
pub const BG2: Color = Color::from_rgb8(0x1A, 0x1E, 0x2B);
pub const BG3: Color = Color::from_rgb8(0x23, 0x28, 0x38);
pub const BG4: Color = Color::from_rgb8(0x2D, 0x33, 0x46);
pub const LINE: Color = Color::from_rgb8(0x2A, 0x30, 0x44);
pub const LINE2: Color = Color::from_rgb8(0x3A, 0x41, 0x59);
pub const TEXT: Color = Color::from_rgb8(0xE6, 0xE9, 0xF2);
pub const TEXT_DIM: Color = Color::from_rgb8(0x9A, 0xA2, 0xB8);
pub const TEXT_FAINT: Color = Color::from_rgb8(0x5B, 0x62, 0x76);
pub const AMBER: Color = Color::from_rgb8(0xF5, 0xB9, 0x42);
pub const GREEN: Color = Color::from_rgb8(0x6F, 0xD4, 0x79);
pub const RED: Color = Color::from_rgb8(0xE5, 0x48, 0x4D);

pub const RAIL_WIDTH: f32 = 68.0;
pub const PANEL_WIDTH: f32 = 280.0;
pub const STATUS_HEIGHT: f32 = 28.0;

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

fn border(color: Color, width: f32, radius: f32) -> Border {
    Border {
        color,
        width,
        radius: radius.into(),
    }
}

fn filled(background: Color) -> container::Style {
    container::Style {
        background: Some(Background::Color(background)),
        text_color: Some(TEXT),
        ..container::Style::default()
    }
}

pub fn rail(_: &Theme) -> container::Style {
    filled(BG0)
}

pub fn panel(_: &Theme) -> container::Style {
    container::Style {
        border: border(LINE, 1.0, 0.0),
        ..filled(BG1)
    }
}

pub fn status_bar(_: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TEXT_DIM),
        ..filled(BG0)
    }
}

pub fn section(_: &Theme) -> container::Style {
    container::Style {
        border: border(LINE, 1.0, 6.0),
        ..filled(BG2)
    }
}

/// The dismissible notice above the main area.
pub fn info_bar(_: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TEXT),
        border: border(AMBER, 1.0, 0.0),
        ..filled(BG3)
    }
}

pub fn modal(_: &Theme) -> container::Style {
    container::Style {
        border: border(LINE2, 1.0, 8.0),
        ..filled(BG2)
    }
}

pub fn backdrop(_: &Theme) -> container::Style {
    filled(Color { a: 0.7, ..BG0 })
}

/// Rail icon button; `selected` draws the amber accent.
pub fn rail_button(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let background = match status {
            button::Status::Hovered | button::Status::Pressed => BG4,
            button::Status::Active | button::Status::Disabled => BG3,
        };
        button::Style {
            background: Some(Background::Color(background)),
            text_color: if matches!(status, button::Status::Disabled) {
                TEXT_FAINT
            } else {
                TEXT
            },
            border: border(
                if selected { AMBER } else { LINE2 },
                if selected { 2.0 } else { 1.0 },
                8.0,
            ),
            ..button::Style::default()
        }
    }
}

/// A destructive action (Remove, Discard): red with readable light text.
pub fn danger_button(_: &Theme, status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Active => RED,
        button::Status::Hovered | button::Status::Pressed => Color {
            r: (RED.r + 0.1).min(1.0),
            g: (RED.g + 0.1).min(1.0),
            b: (RED.b + 0.1).min(1.0),
            ..RED
        },
        button::Status::Disabled => BG3,
    };
    button::Style {
        background: Some(Background::Color(background)),
        text_color: if status == button::Status::Disabled {
            TEXT_FAINT
        } else {
            TEXT
        },
        border: border(background, 1.0, 4.0),
        ..button::Style::default()
    }
}

/// A row in the save list; `current` marks the file open in the editor.
pub fn save_row(current: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let background = match (current, status) {
            (true, _) => BG3,
            (false, button::Status::Hovered | button::Status::Pressed) => BG2,
            (false, button::Status::Active | button::Status::Disabled) => BG1,
        };
        button::Style {
            background: Some(Background::Color(background)),
            text_color: TEXT,
            border: border(if current { AMBER } else { BG1 }, 1.0, 4.0),
            ..button::Style::default()
        }
    }
}

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

pub fn error_text(_: &Theme) -> text::Style {
    text::Style { color: Some(RED) }
}
