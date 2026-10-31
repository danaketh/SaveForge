//! Renders a plugin's UI tree with iced widgets (spec §4.4, §4.5).
//!
//! Styled after the design's editor body: amber section headings, labelled
//! field rows with a hairline between them, steppers for numbers and an
//! underlined tab bar.
//!
//! The tree has already passed `validate_tree`, so child indices are in range
//! and depth is bounded. `scrollable` nodes are rendered as plain containers:
//! the editor pane already scrolls, and nested vertical scrolling is not
//! supported by iced.

use std::fmt;

use iced::widget::{
    Column, Row, button, checkbox, column, container, pick_list, row, rule, slider, space, text,
    text_input,
};
use iced::{Alignment, Element, Length, Padding, Theme};
use saveforge_plugin_api::ui::{
    Button as ButtonSpec, ButtonStyle, Event, Field, Input, ListSpec, Node, NumberInput, PickList,
    PickOption, Tabs, TextSpan, TextStyle, UiTree, Widget,
};

use crate::components::{Size, btn, hline, section_title};
use crate::fonts;
use crate::icons::Icon;
use crate::theme;
use crate::ui_state::{UiState, format_number};

/// Width of a field's label column (design `.field`).
const LABEL_WIDTH: f32 = 180.0;

/// What the rendered UI emits. Only `Plugin` events reach the plugin directly;
/// the other two update host-side [`UiState`] first.
#[derive(Debug, Clone, PartialEq)]
pub enum UiMessage {
    Plugin(Event),
    NumberEdited { id: String, text: String },
    ListChoice { id: String, value: String },
}

/// A pick-list option; displays its label, carries its value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice {
    pub value: String,
    pub display: String,
}

impl From<&PickOption> for Choice {
    fn from(option: &PickOption) -> Self {
        Self {
            value: option.value.clone(),
            display: option.display.clone(),
        }
    }
}

impl fmt::Display for Choice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.display)
    }
}

type ButtonStyleFn = fn(&Theme, button::Status) -> button::Style;

pub fn render<'a>(tree: &'a UiTree, state: &'a UiState) -> Element<'a, UiMessage> {
    node(tree, state, 0)
}

fn node<'a>(tree: &'a UiTree, state: &'a UiState, index: usize) -> Element<'a, UiMessage> {
    let Some(n) = tree.nodes.get(index) else {
        return text("").into();
    };
    let children = || children(tree, state, n);
    match &n.widget {
        Widget::Column => Column::with_children(children()).spacing(8).into(),
        Widget::Row => Row::with_children(children())
            .spacing(8)
            .align_y(Alignment::Center)
            .into(),
        Widget::Scrollable => Column::with_children(children()).into(),
        Widget::Divider => rule::horizontal(1).style(theme::divider).into(),
        Widget::Section(title) => column![
            section_title(title.as_str(), None),
            Column::with_children(children()).spacing(4),
        ]
        .spacing(4)
        .padding(Padding::new(0.0).bottom(20))
        .width(Length::Fill)
        .into(),
        Widget::Tabs(tabs) => tabs_view(tabs, Column::with_children(children()).into()),
        Widget::Text(span) => text_view(span),
        Widget::Field(field) => field_view(field, state),
        Widget::Button(spec) => button_view(spec),
        Widget::List(spec) => list_view(spec, n, tree, state),
    }
}

fn children<'a>(
    tree: &'a UiTree,
    state: &'a UiState,
    n: &'a Node,
) -> impl Iterator<Item = Element<'a, UiMessage>> {
    n.children
        .iter()
        .map(move |&child| node(tree, state, child as usize))
}

/// `.tabbar`: tabs with an amber underline under the selected one.
fn tabs_view<'a>(tabs: &'a Tabs, page: Element<'a, UiMessage>) -> Element<'a, UiMessage> {
    let strip = Row::with_children(tabs.titles.iter().enumerate().map(|(i, title)| {
        let index = u32::try_from(i).unwrap_or(u32::MAX);
        let active = index == tabs.selected;
        let underline = container(space())
            .width(Length::Fill)
            .height(2)
            .style(if active { theme::marker } else { theme::bg1 });
        column![
            button(text(title.as_str()).size(12))
                .padding([8, 16])
                .style(theme::tab(active))
                .on_press(UiMessage::Plugin(Event::TabSelected((
                    tabs.id.clone(),
                    index,
                )))),
            underline,
        ]
        .width(Length::Shrink)
        .into()
    }));
    column![
        column![strip, hline()],
        container(page).padding(Padding::new(0.0).top(12))
    ]
    .into()
}

fn text_view(span: &TextSpan) -> Element<'_, UiMessage> {
    let t = text(span.text.as_str()).size(12);
    match span.style {
        TextStyle::Body => t.into(),
        TextStyle::Muted => t.style(theme::muted).into(),
        TextStyle::Heading => t.size(20).font(fonts::PIXEL).style(theme::heading).into(),
        TextStyle::Error => t.style(theme::error_text).into(),
    }
}

/// `.field`: label (and help) on the left, the input on the right, a hairline
/// below.
fn field_view<'a>(field: &'a Field, state: &'a UiState) -> Element<'a, UiMessage> {
    let editable = !field.read_only;
    let (input, buffer_error): (Element<'a, UiMessage>, Option<String>) = match &field.input {
        Input::Text(t) => {
            let id = field.id.clone();
            let max_len = t.max_len;
            let input = text_input(t.placeholder.as_deref().unwrap_or(""), &t.value)
                .size(12)
                .padding([6, 10])
                .style(theme::input)
                .on_input_maybe(editable.then_some(move |value: String| {
                    UiMessage::Plugin(Event::TextChanged((id.clone(), clamp_len(value, max_len))))
                }));
            (input.into(), None)
        }
        Input::Number(n) => {
            let (shown, error) = state.number_display(&field.id, n);
            (number_view(&field.id, n, &shown, editable), error)
        }
        Input::Checkbox(checked) => {
            let id = field.id.clone();
            let input = checkbox(*checked)
                .size(16)
                .style(theme::check)
                .on_toggle_maybe(editable.then_some(move |value: bool| {
                    UiMessage::Plugin(Event::Toggled((id.clone(), value)))
                }));
            (input.into(), None)
        }
        Input::Pick(list) => (pick_view(&field.id, list, editable), None),
    };

    let mut label = column![text(field.label.as_str()).size(11).style(theme::muted)].spacing(2);
    if let Some(help) = &field.help {
        label = label.push(text(help.as_str()).size(10).style(theme::faint));
    }
    let mut value = column![input].spacing(4);
    if let Some(error) = field.error.clone().or(buffer_error) {
        value = value.push(text(error).size(10).style(theme::error_text));
    }
    column![
        row![
            label.width(Length::Fixed(LABEL_WIDTH)),
            value.width(Length::Fill)
        ]
        .spacing(16)
        .align_y(Alignment::Center),
        hline(),
    ]
    .spacing(10)
    .padding(Padding::new(0.0).top(10))
    .into()
}

/// A stepper (`− value +`), plus a slider when the field has both bounds.
fn number_view<'a>(
    id: &str,
    input: &NumberInput,
    shown: &str,
    editable: bool,
) -> Element<'a, UiMessage> {
    let edited = {
        let id = id.to_owned();
        move |text: String| UiMessage::NumberEdited {
            id: id.clone(),
            text,
        }
    };
    let step = input
        .step
        .unwrap_or_else(|| 10f64.powi(-i32::from(input.decimals)));
    let step_by = |sign: f64| {
        let next = clamp(input.value + sign * step, input);
        (editable && next.to_bits() != input.value.to_bits()).then(|| {
            edited(format_number(&NumberInput {
                value: next,
                ..*input
            }))
        })
    };
    let end = |label: &'static str, message: Option<UiMessage>| {
        button(text(label).size(14).font(fonts::MONO_BOLD).center())
            .width(26)
            .height(Length::Fill)
            .padding(0)
            .style(theme::stepper_end)
            .on_press_maybe(message)
    };
    let value = text_input("", shown)
        .size(12)
        .padding([6, 10])
        .width(96)
        .align_x(Alignment::Center)
        .style(theme::stepper_input)
        .on_input_maybe(editable.then_some(edited.clone()));
    let stepper = container(
        row![
            end("−", step_by(-1.0)),
            container(space())
                .width(1)
                .height(Length::Fill)
                .style(theme::line),
            value,
            container(space())
                .width(1)
                .height(Length::Fill)
                .style(theme::line),
            end("+", step_by(1.0)),
        ]
        .height(Length::Shrink),
    )
    .style(theme::stepper);

    let bounds = input.min.zip(input.max).filter(|(min, max)| max > min);
    let Some((min, max)) = bounds.filter(|_| editable) else {
        return stepper.into();
    };
    let template = *input;
    let bar = slider(min..=max, input.value.clamp(min, max), move |value| {
        edited(format_number(&NumberInput { value, ..template }))
    })
    .step(step)
    .height(20)
    .style(theme::slider_style);
    row![stepper, bar]
        .spacing(14)
        .align_y(Alignment::Center)
        .into()
}

/// `value` within the input's bounds.
fn clamp(value: f64, input: &NumberInput) -> f64 {
    let value = input.min.map_or(value, |min| value.max(min));
    input.max.map_or(value, |max| value.min(max))
}

fn pick_view<'a>(id: &str, list: &'a PickList, editable: bool) -> Element<'a, UiMessage> {
    let options: Vec<Choice> = list.options.iter().map(Choice::from).collect();
    let selected = list
        .selected
        .as_ref()
        .and_then(|value| options.iter().find(|c| &c.value == value).cloned());
    if !editable {
        return text(selected.map(|c| c.display).unwrap_or_default())
            .size(12)
            .into();
    }
    let id = id.to_owned();
    let mut pick = pick_list(options, selected, move |choice: Choice| {
        UiMessage::Plugin(Event::Picked((id.clone(), choice.value)))
    })
    .text_size(12)
    .padding([6, 10])
    .width(Length::Fill)
    .style(theme::pick)
    .menu_style(theme::pick_menu);
    if let Some(placeholder) = &list.placeholder {
        pick = pick.placeholder(placeholder.as_str());
    }
    pick.into()
}

fn button_view(spec: &ButtonSpec) -> Element<'_, UiMessage> {
    let style: ButtonStyleFn = match spec.style {
        ButtonStyle::Primary => theme::btn_primary,
        ButtonStyle::Secondary => theme::btn,
        ButtonStyle::Danger => theme::btn_danger,
    };
    btn(spec.label.as_str(), None, Size::Normal)
        .style(style)
        .on_press_maybe(
            spec.enabled
                .then(|| UiMessage::Plugin(Event::Pressed(spec.id.clone()))),
        )
        .into()
}

fn list_view<'a>(
    spec: &'a ListSpec,
    n: &'a Node,
    tree: &'a UiTree,
    state: &'a UiState,
) -> Element<'a, UiMessage> {
    let rows = children(tree, state, n).enumerate().map(|(i, content)| {
        let content = if spec.removable {
            let index = u32::try_from(i).unwrap_or(u32::MAX);
            row![
                container(content).width(Length::Fill),
                btn("Remove", Some(Icon::Trash), Size::Small)
                    .style(theme::btn_danger)
                    .on_press(UiMessage::Plugin(Event::ListRemove((
                        spec.id.clone(),
                        index
                    )))),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .into()
        } else {
            content
        };
        column![content, hline()].spacing(6).into()
    });
    let mut col = Column::with_children(rows).spacing(6);

    if let Some(options) = &spec.add_options {
        let choices: Vec<Choice> = options.options.iter().map(Choice::from).collect();
        let chosen = state
            .list_choice(&spec.id)
            .and_then(|value| choices.iter().find(|c| c.value == value).cloned());
        let id = spec.id.clone();
        let mut pick = pick_list(choices, chosen.clone(), move |choice: Choice| {
            UiMessage::ListChoice {
                id: id.clone(),
                value: choice.value,
            }
        })
        .text_size(12)
        .padding([6, 10])
        .style(theme::pick)
        .menu_style(theme::pick_menu);
        if let Some(placeholder) = &options.placeholder {
            pick = pick.placeholder(placeholder.as_str());
        }
        let add = btn("Add", Some(Icon::Plus), Size::Normal).on_press_maybe(
            chosen.map(|c| UiMessage::Plugin(Event::ListAdd((spec.id.clone(), c.value)))),
        );
        col = col.push(
            row![pick, add]
                .spacing(8)
                .align_y(Alignment::Center)
                .padding(Padding::new(0.0).top(4)),
        );
    }
    col.into()
}

/// Enforces a text field's `max-len` (in characters) host-side.
fn clamp_len(value: String, max_len: Option<u32>) -> String {
    match max_len.map(|m| m as usize) {
        Some(max) if value.chars().count() > max => value.chars().take(max).collect(),
        _ => value,
    }
}
