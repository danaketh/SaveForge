use saveforge_sdk::ui::{
    ButtonSpec, ButtonStyle, El, FieldSpec, Input, ListSpec, Node, NumberInput, PickList,
    TextInput, TextSpan, TextStyle, UiTree, Widget, button, checkbox, column, divider, heading,
    list, muted, number, option, pick, scrollable, section, tabs, text, text_input,
};
use saveforge_sdk::{col, row};

fn span(text: &str, style: TextStyle) -> Widget {
    Widget::Text(TextSpan {
        text: text.to_owned(),
        style,
    })
}

#[test]
fn flattens_pre_order_with_root_first() {
    let tree = col![
        heading("Title"),
        section("Stats", vec![text("a"), muted("b")]),
        divider(),
    ]
    .into_tree();
    assert_eq!(
        tree,
        UiTree {
            nodes: vec![
                Node {
                    widget: Widget::Column,
                    children: vec![1, 2, 5]
                },
                Node {
                    widget: span("Title", TextStyle::Heading),
                    children: vec![]
                },
                Node {
                    widget: Widget::Section("Stats".to_owned()),
                    children: vec![3, 4]
                },
                Node {
                    widget: span("a", TextStyle::Body),
                    children: vec![]
                },
                Node {
                    widget: span("b", TextStyle::Muted),
                    children: vec![]
                },
                Node {
                    widget: Widget::Divider,
                    children: vec![]
                },
            ]
        }
    );
}

#[test]
fn number_field_builder() {
    let el: El = number("hp", "Health", 42.5)
        .range(0.0, 100.0)
        .decimals(1)
        .step(0.5)
        .help("0-100")
        .into();
    let tree = el.into_tree();
    assert_eq!(
        tree.nodes[0].widget,
        Widget::Field(FieldSpec {
            id: "hp".to_owned(),
            label: "Health".to_owned(),
            help: Some("0-100".to_owned()),
            error: None,
            read_only: false,
            input: Input::Number(NumberInput {
                value: 42.5,
                min: Some(0.0),
                max: Some(100.0),
                decimals: 1,
                step: Some(0.5),
            }),
        })
    );
}

#[test]
fn text_field_builder_and_inapplicable_methods_are_noops() {
    let el: El = text_input("name", "Name", "Bob")
        .placeholder("type")
        .max_len(64)
        .range(1.0, 2.0)
        .read_only(true)
        .error("bad")
        .into();
    let Widget::Field(field) = &el.into_tree().nodes[0].widget else {
        panic!("expected field");
    };
    assert!(field.read_only);
    assert_eq!(field.error.as_deref(), Some("bad"));
    assert_eq!(
        field.input,
        Input::Text(TextInput {
            value: "Bob".to_owned(),
            placeholder: Some("type".to_owned()),
            max_len: Some(64),
        })
    );
}

#[test]
fn checkbox_pick_and_button_builders() {
    let tree = row![
        checkbox("god", "God mode", true),
        pick(
            "char",
            "Character",
            vec![option("0", "Ann"), option("1", "Bo")],
            Some("1".to_owned())
        )
        .placeholder("choose"),
        button("save", "Save")
            .style(ButtonStyle::Danger)
            .enabled(false),
    ]
    .into_tree();
    assert_eq!(tree.nodes[0].widget, Widget::Row);
    let Widget::Field(cb) = &tree.nodes[1].widget else {
        panic!("expected checkbox field")
    };
    assert_eq!(cb.input, Input::Checkbox(true));
    let Widget::Field(pk) = &tree.nodes[2].widget else {
        panic!("expected pick field")
    };
    let Input::Pick(PickList {
        options,
        selected,
        placeholder,
    }) = &pk.input
    else {
        panic!("expected pick input")
    };
    assert_eq!(options.len(), 2);
    assert_eq!(selected.as_deref(), Some("1"));
    assert_eq!(placeholder.as_deref(), Some("choose"));
    assert_eq!(
        tree.nodes[3].widget,
        Widget::Button(ButtonSpec {
            id: "save".to_owned(),
            label: "Save".to_owned(),
            style: ButtonStyle::Danger,
            enabled: false,
        })
    );
}

#[test]
fn tabs_and_scrollable_wrap_exactly_one_child() {
    let tree = tabs(
        "main",
        vec!["A".to_owned(), "B".to_owned()],
        1,
        scrollable(column(vec![text("page")])),
    )
    .into_tree();
    assert_eq!(tree.nodes[0].children, vec![1]);
    assert_eq!(tree.nodes[1].widget, Widget::Scrollable);
    assert_eq!(tree.nodes[1].children, vec![2]);
    assert_eq!(tree.nodes.len(), 4);
}

#[test]
fn list_builder() {
    let tree: UiTree = El::from(
        list("inv", vec![text("Iron"), text("Ice")])
            .removable(true)
            .add_options(vec![option("iron", "Iron")], Some("Add item".to_owned())),
    )
    .into_tree();
    assert_eq!(tree.nodes[0].children, vec![1, 2]);
    assert_eq!(
        tree.nodes[0].widget,
        Widget::List(ListSpec {
            id: "inv".to_owned(),
            removable: true,
            add_options: Some(PickList {
                options: vec![option("iron", "Iron")],
                selected: None,
                placeholder: Some("Add item".to_owned()),
            }),
        })
    );
}
