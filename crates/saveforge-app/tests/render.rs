//! Renderer tests through iced's headless simulator.

use iced_test::simulator;
use saveforge_app::render::{UiMessage, render};
use saveforge_app::ui_state::UiState;
use saveforge_plugin_api::ui::{
    Button, ButtonStyle, Event, Field, Input, ListSpec, Node, NumberInput, PickList, PickOption,
    Tabs, TextInput, TextSpan, TextStyle, UiTree, Widget,
};

fn leaf(widget: Widget) -> Node {
    Node {
        widget,
        children: vec![],
    }
}

fn text(s: &str) -> Widget {
    Widget::Text(TextSpan {
        text: s.to_owned(),
        style: TextStyle::Body,
    })
}

fn field(id: &str, label: &str, read_only: bool, input: Input) -> Widget {
    Widget::Field(Field {
        id: id.to_owned(),
        label: label.to_owned(),
        help: None,
        error: None,
        read_only,
        input,
    })
}

fn text_input(placeholder: &str, max_len: Option<u32>) -> Input {
    Input::Text(TextInput {
        value: String::new(),
        placeholder: Some(placeholder.to_owned()),
        max_len,
    })
}

fn button(id: &str, label: &str, enabled: bool) -> Widget {
    Widget::Button(Button {
        id: id.to_owned(),
        label: label.to_owned(),
        style: ButtonStyle::Primary,
        enabled,
    })
}

/// A column holding `nodes` as leaves.
fn column(nodes: Vec<Widget>) -> UiTree {
    let children = (1..=u32::try_from(nodes.len()).unwrap()).collect();
    let mut all = vec![Node {
        widget: Widget::Column,
        children,
    }];
    all.extend(nodes.into_iter().map(leaf));
    UiTree { nodes: all }
}

fn messages(
    tree: &UiTree,
    state: &UiState,
    act: impl FnOnce(&mut iced_test::Simulator<'_, UiMessage>),
) -> Vec<UiMessage> {
    let mut ui = simulator(render(tree, state));
    act(&mut ui);
    ui.into_messages().collect()
}

#[test]
fn shows_text_labels_and_number_values() {
    let tree = column(vec![
        text("Hello"),
        field(
            "hp",
            "Health",
            false,
            Input::Number(NumberInput {
                value: 42.0,
                min: None,
                max: None,
                decimals: 0,
                step: None,
            }),
        ),
    ]);
    let state = UiState::default();
    let mut ui = simulator(render(&tree, &state));
    assert!(ui.find("Hello").is_ok());
    assert!(ui.find("Health").is_ok());
    assert!(ui.find("42").is_ok());
}

#[test]
fn enabled_button_emits_pressed_disabled_does_not() {
    let state = UiState::default();
    let tree = column(vec![
        button("go", "Go", true),
        button("stop", "Stop", false),
    ]);
    let got = messages(&tree, &state, |ui| {
        let _ = ui.click("Go");
        let _ = ui.click("Stop");
    });
    assert_eq!(got, [UiMessage::Plugin(Event::Pressed("go".to_owned()))]);
}

#[test]
fn text_field_emits_changes_clamped_to_max_len() {
    let state = UiState::default();
    let tree = column(vec![field(
        "name",
        "Name",
        false,
        text_input("type name", Some(3)),
    )]);
    let got = messages(&tree, &state, |ui| {
        let _ = ui.click("type name");
        let _ = ui.typewrite("abcd");
    });
    assert_eq!(
        got.last(),
        Some(&UiMessage::Plugin(Event::TextChanged((
            "name".to_owned(),
            "abc".to_owned()
        ))))
    );
}

#[test]
fn read_only_text_field_emits_nothing() {
    let state = UiState::default();
    let tree = column(vec![field(
        "name",
        "Name",
        true,
        text_input("type name", None),
    )]);
    let got = messages(&tree, &state, |ui| {
        let _ = ui.click("type name");
        let _ = ui.typewrite("x");
    });
    assert_eq!(got, []);
}

#[test]
fn number_field_emits_raw_text_and_shows_buffer_error() {
    let input = NumberInput {
        value: 1.0,
        min: Some(0.0),
        max: None,
        decimals: 0,
        step: None,
    };
    let tree = column(vec![field("hp", "Health", false, Input::Number(input))]);
    let mut state = UiState::default();
    let got = messages(&tree, &state, |ui| {
        let _ = ui.click("1");
        let _ = ui.typewrite("5");
    });
    assert!(
        matches!(got.last(), Some(UiMessage::NumberEdited { id, .. }) if id == "hp"),
        "{got:?}"
    );

    state.edit_number("hp", "-4".to_owned(), &input);
    let mut ui = simulator(render(&tree, &state));
    assert!(ui.find("-4").is_ok());
    assert!(ui.find("Must be at least 0").is_ok());
}

#[test]
fn tab_click_emits_tab_selected() {
    let tree = UiTree {
        nodes: vec![
            Node {
                widget: Widget::Tabs(Tabs {
                    id: "main".to_owned(),
                    titles: vec!["Stats".to_owned(), "Items".to_owned()],
                    selected: 0,
                }),
                children: vec![1],
            },
            leaf(text("page")),
        ],
    };
    let state = UiState::default();
    let got = messages(&tree, &state, |ui| {
        let _ = ui.click("Items");
    });
    assert_eq!(
        got,
        [UiMessage::Plugin(Event::TabSelected((
            "main".to_owned(),
            1
        )))]
    );
}

#[test]
fn list_rows_have_remove_buttons() {
    let tree = UiTree {
        nodes: vec![
            Node {
                widget: Widget::List(ListSpec {
                    id: "inv".to_owned(),
                    removable: true,
                    add_options: Some(PickList {
                        options: vec![PickOption {
                            value: "ice".to_owned(),
                            display: "Ice".to_owned(),
                        }],
                        selected: None,
                        placeholder: Some("Add item".to_owned()),
                    }),
                }),
                children: vec![1, 2],
            },
            leaf(text("Iron")),
            leaf(text("Water")),
        ],
    };
    let state = UiState::default();
    let mut ui = simulator(render(&tree, &state));
    assert!(ui.find("Water").is_ok());
    let _ = ui.click("Remove");
    let _ = ui.click("Add");
    let got: Vec<UiMessage> = ui.into_messages().collect();
    // "Add" is disabled until a choice is made, so only the removal arrives.
    assert_eq!(
        got,
        [UiMessage::Plugin(Event::ListRemove(("inv".to_owned(), 0)))]
    );
}
