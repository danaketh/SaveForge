//! Renders a plugin's UI tree with iced widgets (spec §4.4, §4.5).
//!
//! The tree has already passed `validate_tree`, so child indices are in range
//! and depth is bounded. `scrollable` nodes are rendered as plain containers:
//! the editor pane already scrolls, and nested vertical scrolling is not
//! supported by iced.

use std::fmt;

use iced::widget::{
    Column, Row, button, checkbox, column, container, pick_list, row, rule, text, text_input,
};
use iced::{Alignment, Element, Length, Theme};
use saveforge_plugin_api::ui::{
    Button as ButtonSpec, ButtonStyle, Event, Field, Input, ListSpec, Node, PickList, PickOption,
    Tabs, TextSpan, TextStyle, UiTree, Widget,
};

use crate::theme;
use crate::ui_state::UiState;

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
        Widget::Divider => rule::horizontal(1).into(),
        Widget::Section(title) => container(
            column![
                text(title.as_str()).size(15).style(theme::heading),
                Column::with_children(children()).spacing(8),
            ]
            .spacing(8),
        )
        .padding(12)
        .width(Length::Fill)
        .style(theme::section)
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

fn tabs_view<'a>(tabs: &'a Tabs, page: Element<'a, UiMessage>) -> Element<'a, UiMessage> {
    let strip = Row::with_children(tabs.titles.iter().enumerate().map(|(i, title)| {
        let index = u32::try_from(i).unwrap_or(u32::MAX);
        let style: ButtonStyleFn = if index == tabs.selected {
            button::primary
        } else {
            button::secondary
        };
        button(text(title.as_str()))
            .style(style)
            .on_press(UiMessage::Plugin(Event::TabSelected((
                tabs.id.clone(),
                index,
            ))))
            .into()
    }))
    .spacing(4);
    column![strip, page].spacing(12).into()
}

fn text_view(span: &TextSpan) -> Element<'_, UiMessage> {
    let t = text(span.text.as_str());
    match span.style {
        TextStyle::Body => t.into(),
        TextStyle::Muted => t.style(theme::muted).into(),
        TextStyle::Heading => t.size(18).style(theme::heading).into(),
        TextStyle::Error => t.style(theme::error_text).into(),
    }
}

fn field_view<'a>(field: &'a Field, state: &'a UiState) -> Element<'a, UiMessage> {
    let editable = !field.read_only;
    let (input, buffer_error): (Element<'a, UiMessage>, Option<String>) = match &field.input {
        Input::Text(t) => {
            let id = field.id.clone();
            let max_len = t.max_len;
            let input = text_input(t.placeholder.as_deref().unwrap_or(""), &t.value)
                .on_input_maybe(editable.then_some(move |value: String| {
                    UiMessage::Plugin(Event::TextChanged((id.clone(), clamp_len(value, max_len))))
                }));
            (input.into(), None)
        }
        Input::Number(n) => {
            let (shown, error) = state.number_display(&field.id, n);
            let id = field.id.clone();
            let input =
                text_input("", &shown).on_input_maybe(editable.then_some(move |value: String| {
                    UiMessage::NumberEdited {
                        id: id.clone(),
                        text: value,
                    }
                }));
            (input.into(), error)
        }
        Input::Checkbox(checked) => {
            let id = field.id.clone();
            let input =
                checkbox(*checked).on_toggle_maybe(editable.then_some(move |value: bool| {
                    UiMessage::Plugin(Event::Toggled((id.clone(), value)))
                }));
            (input.into(), None)
        }
        Input::Pick(list) => (pick_view(&field.id, list, editable), None),
    };

    let mut col = column![
        row![
            text(field.label.as_str()).width(Length::Fixed(180.0)),
            container(input).width(Length::Fill),
        ]
        .spacing(12)
        .align_y(Alignment::Center)
    ]
    .spacing(4);
    if let Some(help) = &field.help {
        col = col.push(text(help.as_str()).size(12).style(theme::muted));
    }
    if let Some(error) = field.error.clone().or(buffer_error) {
        col = col.push(text(error).size(12).style(theme::error_text));
    }
    col.into()
}

fn pick_view<'a>(id: &str, list: &'a PickList, editable: bool) -> Element<'a, UiMessage> {
    let options: Vec<Choice> = list.options.iter().map(Choice::from).collect();
    let selected = list
        .selected
        .as_ref()
        .and_then(|value| options.iter().find(|c| &c.value == value).cloned());
    if !editable {
        return text(selected.map(|c| c.display).unwrap_or_default()).into();
    }
    let id = id.to_owned();
    let mut pick = pick_list(options, selected, move |choice: Choice| {
        UiMessage::Plugin(Event::Picked((id.clone(), choice.value)))
    });
    if let Some(placeholder) = &list.placeholder {
        pick = pick.placeholder(placeholder.as_str());
    }
    pick.into()
}

fn button_view(spec: &ButtonSpec) -> Element<'_, UiMessage> {
    let style: ButtonStyleFn = match spec.style {
        ButtonStyle::Primary => button::primary,
        ButtonStyle::Secondary => button::secondary,
        ButtonStyle::Danger => button::danger,
    };
    button(text(spec.label.as_str()))
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
        if spec.removable {
            let index = u32::try_from(i).unwrap_or(u32::MAX);
            row![
                container(content).width(Length::Fill),
                button(text("Remove"))
                    .style(button::danger)
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
        }
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
        });
        if let Some(placeholder) = &options.placeholder {
            pick = pick.placeholder(placeholder.as_str());
        }
        let add = button(text("Add")).on_press_maybe(
            chosen.map(|c| UiMessage::Plugin(Event::ListAdd((spec.id.clone(), c.value)))),
        );
        col = col.push(row![pick, add].spacing(8));
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
