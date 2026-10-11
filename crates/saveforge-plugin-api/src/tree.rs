//! Structural checks on a plugin's UI tree (spec §4.5).

use std::collections::HashSet;

use crate::ui::{UiTree, Widget};

pub const MAX_TREE_NODES: usize = 20_000;

/// Checks that `tree` is a well-formed tree the renderer can trust. Returns a
/// human-readable reason on failure.
///
/// Rules: non-empty and at most [`MAX_TREE_NODES`] nodes. Every child index is
/// in range and not the root. Every non-root node has exactly one parent and is
/// reachable from the root, which rules out cycles. Child counts fit the widget
/// (containers: any, `scrollable`/`tabs`: exactly one, leaves: none). Widget ids
/// are unique. `tabs` has at least one title and `selected` is in range.
pub fn validate_tree(tree: &UiTree) -> Result<(), String> {
    let nodes = &tree.nodes;
    if nodes.is_empty() {
        return Err("tree has no root node".to_owned());
    }
    if nodes.len() > MAX_TREE_NODES {
        return Err(format!(
            "tree has {} nodes (max {MAX_TREE_NODES})",
            nodes.len()
        ));
    }

    let mut parent: Vec<Option<usize>> = vec![None; nodes.len()];
    let mut ids = HashSet::new();
    for (index, node) in nodes.iter().enumerate() {
        check_child_count(index, &node.widget, node.children.len())?;
        if let Some(id) = widget_id(&node.widget)
            && !ids.insert(id)
        {
            return Err(format!("duplicate widget id '{id}'"));
        }
        if let Widget::Tabs(tabs) = &node.widget
            && (tabs.titles.is_empty() || tabs.selected as usize >= tabs.titles.len())
        {
            return Err(format!(
                "tabs '{}' has {} titles but selected index {}",
                tabs.id,
                tabs.titles.len(),
                tabs.selected
            ));
        }
        for &child in &node.children {
            let child = child as usize;
            if child == 0 || child >= nodes.len() {
                return Err(format!("node {index} has invalid child index {child}"));
            }
            if let Some(first) = parent[child] {
                return Err(format!(
                    "node {child} has two parents ({first} and {index})"
                ));
            }
            parent[child] = Some(index);
        }
    }

    let mut seen = vec![false; nodes.len()];
    let mut stack = vec![0usize];
    while let Some(index) = stack.pop() {
        if std::mem::replace(&mut seen[index], true) {
            continue;
        }
        stack.extend(nodes[index].children.iter().map(|&c| c as usize));
    }
    if let Some(orphan) = seen.iter().position(|reached| !reached) {
        return Err(format!("node {orphan} is not reachable from the root"));
    }
    Ok(())
}

fn check_child_count(index: usize, widget: &Widget, count: usize) -> Result<(), String> {
    let ok = match widget {
        Widget::Column | Widget::Row | Widget::Section(_) | Widget::List(_) => true,
        Widget::Scrollable | Widget::Tabs(_) => count == 1,
        Widget::Divider | Widget::Text(_) | Widget::Field(_) | Widget::Button(_) => count == 0,
    };
    if ok {
        Ok(())
    } else {
        Err(format!(
            "node {index} ({}) has {count} children",
            widget_kind(widget)
        ))
    }
}

fn widget_id(widget: &Widget) -> Option<&str> {
    match widget {
        Widget::Tabs(tabs) => Some(&tabs.id),
        Widget::Field(field) => Some(&field.id),
        Widget::Button(button) => Some(&button.id),
        Widget::List(list) => Some(&list.id),
        Widget::Column
        | Widget::Row
        | Widget::Scrollable
        | Widget::Divider
        | Widget::Section(_)
        | Widget::Text(_) => None,
    }
}

fn widget_kind(widget: &Widget) -> &'static str {
    match widget {
        Widget::Column => "column",
        Widget::Row => "row",
        Widget::Scrollable => "scrollable",
        Widget::Divider => "divider",
        Widget::Section(_) => "section",
        Widget::Tabs(_) => "tabs",
        Widget::Text(_) => "text",
        Widget::Field(_) => "field",
        Widget::Button(_) => "button",
        Widget::List(_) => "list",
    }
}
