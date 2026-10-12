use saveforge_app::ui_state::{UiState, find_number_input, format_number, parse_number};
use saveforge_plugin_api::ui::{
    Field, Input, ListSpec, Node, NumberInput, TextInput, UiTree, Widget,
};

fn number(value: f64, min: Option<f64>, max: Option<f64>, decimals: u8) -> NumberInput {
    NumberInput {
        value,
        min,
        max,
        decimals,
        step: None,
    }
}

fn field(id: &str, input: Input) -> Node {
    Node {
        widget: Widget::Field(Field {
            id: id.to_owned(),
            label: id.to_owned(),
            help: None,
            error: None,
            read_only: false,
            input,
        }),
        children: vec![],
    }
}

/// A column holding `nodes` as its direct children.
fn tree(nodes: Vec<Node>) -> UiTree {
    let children = (1..=u32::try_from(nodes.len()).unwrap()).collect();
    let mut all = vec![Node {
        widget: Widget::Column,
        children,
    }];
    all.extend(nodes);
    UiTree { nodes: all }
}

#[test]
fn formats_with_decimals() {
    assert_eq!(format_number(&number(42.0, None, None, 0)), "42");
    assert_eq!(format_number(&number(1.23456, None, None, 2)), "1.23");
}

#[test]
fn parse_rules() {
    let hp = number(50.0, Some(0.0), Some(100.0), 1);
    assert_eq!(parse_number(" 12.5 ", &hp), Ok(12.5));
    assert_eq!(parse_number("12.", &hp), Ok(12.0));
    assert_eq!(parse_number("abc", &hp), Err("Enter a number".to_owned()));
    assert_eq!(parse_number("NaN", &hp), Err("Enter a number".to_owned()));
    assert_eq!(parse_number("inf", &hp), Err("Enter a number".to_owned()));
    assert_eq!(
        parse_number("1.25", &hp),
        Err("At most 1 decimal places".to_owned())
    );
    assert_eq!(
        parse_number("-1", &hp),
        Err("Must be at least 0".to_owned())
    );
    assert_eq!(
        parse_number("100.5", &hp),
        Err("Must be at most 100".to_owned())
    );
    let whole = number(1.0, None, None, 0);
    assert_eq!(
        parse_number("1.5", &whole),
        Err("Whole numbers only".to_owned())
    );
}

#[test]
fn valid_edit_is_sent_and_text_kept_while_plugin_agrees() {
    let input = number(5.0, None, None, 1);
    let mut state = UiState::default();
    assert_eq!(state.edit_number("hp", "7.".to_owned(), &input), Some(7.0));
    // The plugin accepted 7.0: the user's "7." stays on screen.
    assert_eq!(
        state.number_display("hp", &number(7.0, None, None, 1)),
        ("7.".to_owned(), None)
    );
    // The plugin changed the value itself: its value wins.
    assert_eq!(
        state.number_display("hp", &number(9.0, None, None, 1)),
        ("9.0".to_owned(), None)
    );
}

#[test]
fn invalid_edit_is_kept_with_error_and_not_sent() {
    let input = number(5.0, Some(0.0), None, 0);
    let mut state = UiState::default();
    assert_eq!(state.edit_number("hp", "-3".to_owned(), &input), None);
    assert_eq!(
        state.number_display("hp", &input),
        ("-3".to_owned(), Some("Must be at least 0".to_owned()))
    );
}

#[test]
fn unknown_field_shows_plugin_value() {
    let state = UiState::default();
    assert_eq!(
        state.number_display("x", &number(1.5, None, None, 2)),
        ("1.50".to_owned(), None)
    );
}

#[test]
fn finds_number_inputs_only() {
    let t = tree(vec![
        field("hp", Input::Number(number(1.0, None, None, 0))),
        field(
            "name",
            Input::Text(TextInput {
                value: String::new(),
                placeholder: None,
                max_len: None,
            }),
        ),
    ]);
    assert_eq!(find_number_input(&t, "hp").map(|n| n.value), Some(1.0));
    assert_eq!(find_number_input(&t, "name"), None);
    assert_eq!(find_number_input(&t, "missing"), None);
}

#[test]
fn retain_drops_state_of_vanished_widgets() {
    let input = number(5.0, None, None, 0);
    let mut state = UiState::default();
    state.edit_number("hp", "bad".to_owned(), &input);
    state.set_list_choice("inv", "iron".to_owned());

    let list = Node {
        widget: Widget::List(ListSpec {
            id: "inv".to_owned(),
            removable: false,
            add_options: None,
        }),
        children: vec![],
    };
    state.retain(&tree(vec![field("hp", Input::Number(input)), list]));
    assert_eq!(state.list_choice("inv"), Some("iron"));
    assert_eq!(state.number_display("hp", &input).0, "bad");

    state.retain(&tree(vec![]));
    assert_eq!(state.list_choice("inv"), None);
    assert_eq!(state.number_display("hp", &input).0, "5");
}
