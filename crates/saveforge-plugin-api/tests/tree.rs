use saveforge_plugin_api::ui::{
    Button, ButtonStyle, Node, Tabs, TextSpan, TextStyle, UiTree, Widget,
};
use saveforge_plugin_api::{API_VERSION, MAX_TREE_NODES, validate_tree};

fn node(widget: Widget, children: &[u32]) -> Node {
    Node {
        widget,
        children: children.to_vec(),
    }
}

fn text(s: &str) -> Widget {
    Widget::Text(TextSpan {
        text: s.to_owned(),
        style: TextStyle::Body,
    })
}

fn button(id: &str) -> Widget {
    Widget::Button(Button {
        id: id.to_owned(),
        label: id.to_owned(),
        style: ButtonStyle::Primary,
        enabled: true,
    })
}

fn tabs(id: &str, titles: usize, selected: u32) -> Widget {
    Widget::Tabs(Tabs {
        id: id.to_owned(),
        titles: (0..titles).map(|i| format!("Tab {i}")).collect(),
        selected,
    })
}

fn tree(nodes: Vec<Node>) -> UiTree {
    UiTree { nodes }
}

fn err(t: &UiTree) -> String {
    validate_tree(t).expect_err("tree should be invalid")
}

#[test]
fn accepts_well_formed_tree() {
    // 0 column -> [1 text, 2 button a, 3 tabs t -> [4 column -> [5 button b]]]
    let t = tree(vec![
        node(Widget::Column, &[1, 2, 3]),
        node(text("hi"), &[]),
        node(button("a"), &[]),
        node(tabs("t", 2, 1), &[4]),
        node(Widget::Column, &[5]),
        node(button("b"), &[]),
    ]);
    assert_eq!(validate_tree(&t), Ok(()));
}

#[test]
fn rejects_empty_tree() {
    assert!(err(&tree(vec![])).contains("root"));
}

#[test]
fn rejects_bad_child_indices() {
    assert!(err(&tree(vec![node(Widget::Column, &[5])])).contains("invalid child"));
    assert!(err(&tree(vec![node(Widget::Column, &[0])])).contains("invalid child"));
}

#[test]
fn rejects_node_with_two_parents() {
    let t = tree(vec![
        node(Widget::Column, &[1, 2]),
        node(Widget::Row, &[3]),
        node(Widget::Row, &[3]),
        node(text("x"), &[]),
    ]);
    assert!(err(&t).contains("two parents"));
}

#[test]
fn rejects_orphans_and_detached_cycles() {
    let orphan = tree(vec![node(Widget::Column, &[]), node(text("lost"), &[])]);
    assert!(err(&orphan).contains("not reachable"));
    let cycle = tree(vec![
        node(Widget::Column, &[]),
        node(Widget::Row, &[2]),
        node(Widget::Row, &[1]),
    ]);
    assert!(err(&cycle).contains("not reachable"));
}

#[test]
fn enforces_child_counts() {
    let leaf_with_child = tree(vec![
        node(Widget::Column, &[1]),
        node(text("x"), &[2]),
        node(text("y"), &[]),
    ]);
    assert!(err(&leaf_with_child).contains("children"));
    let tabs_without_page = tree(vec![node(tabs("t", 1, 0), &[])]);
    assert!(err(&tabs_without_page).contains("children"));
    let scroll_two = tree(vec![
        node(Widget::Scrollable, &[1, 2]),
        node(text("a"), &[]),
        node(text("b"), &[]),
    ]);
    assert!(err(&scroll_two).contains("children"));
}

#[test]
fn rejects_duplicate_ids() {
    let t = tree(vec![
        node(Widget::Column, &[1, 2]),
        node(button("same"), &[]),
        node(button("same"), &[]),
    ]);
    assert!(err(&t).contains("duplicate"));
}

#[test]
fn rejects_bad_tabs() {
    let out_of_range = tree(vec![node(tabs("t", 2, 2), &[1]), node(text("p"), &[])]);
    assert!(err(&out_of_range).contains("tabs"));
    let no_titles = tree(vec![node(tabs("t", 0, 0), &[1]), node(text("p"), &[])]);
    assert!(err(&no_titles).contains("tabs"));
}

#[test]
fn node_cap_is_inclusive() {
    let at_cap: Vec<u32> = (1..u32::try_from(MAX_TREE_NODES).unwrap()).collect();
    let mut nodes = vec![node(Widget::Column, &at_cap)];
    nodes.extend((1..MAX_TREE_NODES).map(|_| node(Widget::Divider, &[])));
    assert_eq!(nodes.len(), MAX_TREE_NODES);
    assert_eq!(validate_tree(&tree(nodes.clone())), Ok(()));

    nodes.push(node(Widget::Divider, &[]));
    nodes[0]
        .children
        .push(u32::try_from(MAX_TREE_NODES).unwrap());
    assert!(err(&tree(nodes)).contains("max"));
}

#[test]
fn api_version_matches_wit_package() {
    let wit = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../wit/plugin.wit"))
        .unwrap();
    let expected = format!("package saveforge:plugin@{API_VERSION};");
    assert!(
        wit.contains(&expected),
        "wit/plugin.wit must declare {expected}"
    );
}
