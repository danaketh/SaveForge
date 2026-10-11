//! UI-tree types and builders (spec §4.4, §4.6).
//!
//! Build a nested [`El`] with the functions here (or the crate-root `col!` /
//! `row!` macros) and call [`El::into_tree`] in `Document::view`.

pub use crate::bindings::saveforge::plugin::ui::{
    Button as ButtonSpec, ButtonStyle, Event, Field as FieldSpec, Input, ListSpec, Node,
    NumberInput, PickList, PickOption, Tabs, TextInput, TextSpan, TextStyle, UiTree, Widget,
};

/// A widget with its children, before flattening into the host's arena form.
#[derive(Debug, Clone, PartialEq)]
pub struct El {
    widget: Widget,
    children: Vec<El>,
}

impl El {
    fn leaf(widget: Widget) -> Self {
        Self {
            widget,
            children: Vec::new(),
        }
    }

    /// Flattens to the arena form: pre-order, root at index 0.
    pub fn into_tree(self) -> UiTree {
        let mut nodes = Vec::new();
        push(self, &mut nodes);
        UiTree { nodes }
    }
}

fn push(el: El, nodes: &mut Vec<Node>) -> u32 {
    let index = nodes.len();
    nodes.push(Node {
        widget: el.widget,
        children: Vec::new(),
    });
    let children = el.children.into_iter().map(|c| push(c, nodes)).collect();
    nodes[index].children = children;
    u32::try_from(index).expect("UI tree exceeds u32::MAX nodes")
}

pub fn column(children: Vec<El>) -> El {
    El {
        widget: Widget::Column,
        children,
    }
}

pub fn row(children: Vec<El>) -> El {
    El {
        widget: Widget::Row,
        children,
    }
}

pub fn scrollable(child: impl Into<El>) -> El {
    El {
        widget: Widget::Scrollable,
        children: vec![child.into()],
    }
}

pub fn divider() -> El {
    El::leaf(Widget::Divider)
}

pub fn section(title: impl Into<String>, children: Vec<El>) -> El {
    El {
        widget: Widget::Section(title.into()),
        children,
    }
}

/// Tab strip; `page` is the content of the `selected` tab only.
pub fn tabs(id: impl Into<String>, titles: Vec<String>, selected: u32, page: impl Into<El>) -> El {
    El {
        widget: Widget::Tabs(Tabs {
            id: id.into(),
            titles,
            selected,
        }),
        children: vec![page.into()],
    }
}

fn styled(text: impl Into<String>, style: TextStyle) -> El {
    El::leaf(Widget::Text(TextSpan {
        text: text.into(),
        style,
    }))
}

pub fn text(text: impl Into<String>) -> El {
    styled(text, TextStyle::Body)
}

pub fn muted(text: impl Into<String>) -> El {
    styled(text, TextStyle::Muted)
}

pub fn heading(text: impl Into<String>) -> El {
    styled(text, TextStyle::Heading)
}

pub fn error_text(text: impl Into<String>) -> El {
    styled(text, TextStyle::Error)
}

pub fn option(value: impl Into<String>, display: impl Into<String>) -> PickOption {
    PickOption {
        value: value.into(),
        display: display.into(),
    }
}

/// Labelled input under construction.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldEl(FieldSpec);

fn field(id: impl Into<String>, label: impl Into<String>, input: Input) -> FieldEl {
    FieldEl(FieldSpec {
        id: id.into(),
        label: label.into(),
        help: None,
        error: None,
        read_only: false,
        input,
    })
}

pub fn text_input(
    id: impl Into<String>,
    label: impl Into<String>,
    value: impl Into<String>,
) -> FieldEl {
    field(
        id,
        label,
        Input::Text(TextInput {
            value: value.into(),
            placeholder: None,
            max_len: None,
        }),
    )
}

pub fn number(id: impl Into<String>, label: impl Into<String>, value: f64) -> FieldEl {
    field(
        id,
        label,
        Input::Number(NumberInput {
            value,
            min: None,
            max: None,
            decimals: 0,
            step: None,
        }),
    )
}

pub fn checkbox(id: impl Into<String>, label: impl Into<String>, checked: bool) -> FieldEl {
    field(id, label, Input::Checkbox(checked))
}

pub fn pick(
    id: impl Into<String>,
    label: impl Into<String>,
    options: Vec<PickOption>,
    selected: Option<String>,
) -> FieldEl {
    field(
        id,
        label,
        Input::Pick(PickList {
            options,
            selected,
            placeholder: None,
        }),
    )
}

impl FieldEl {
    #[must_use]
    pub fn help(mut self, help: impl Into<String>) -> Self {
        self.0.help = Some(help.into());
        self
    }

    #[must_use]
    pub fn error(mut self, error: impl Into<String>) -> Self {
        self.0.error = Some(error.into());
        self
    }

    #[must_use]
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.0.read_only = read_only;
        self
    }

    /// Text and pick inputs only.
    #[must_use]
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        match &mut self.0.input {
            Input::Text(t) => t.placeholder = Some(placeholder.into()),
            Input::Pick(p) => p.placeholder = Some(placeholder.into()),
            Input::Number(_) | Input::Checkbox(_) => {}
        }
        self
    }

    /// Text inputs only.
    #[must_use]
    pub fn max_len(mut self, max_len: u32) -> Self {
        if let Input::Text(t) = &mut self.0.input {
            t.max_len = Some(max_len);
        }
        self
    }

    /// Number inputs only.
    #[must_use]
    pub fn range(self, min: f64, max: f64) -> Self {
        self.min(min).max(max)
    }

    /// Number inputs only.
    #[must_use]
    pub fn min(mut self, min: f64) -> Self {
        if let Input::Number(n) = &mut self.0.input {
            n.min = Some(min);
        }
        self
    }

    /// Number inputs only.
    #[must_use]
    pub fn max(mut self, max: f64) -> Self {
        if let Input::Number(n) = &mut self.0.input {
            n.max = Some(max);
        }
        self
    }

    /// Number inputs only.
    #[must_use]
    pub fn decimals(mut self, decimals: u8) -> Self {
        if let Input::Number(n) = &mut self.0.input {
            n.decimals = decimals;
        }
        self
    }

    /// Number inputs only.
    #[must_use]
    pub fn step(mut self, step: f64) -> Self {
        if let Input::Number(n) = &mut self.0.input {
            n.step = Some(step);
        }
        self
    }
}

impl From<FieldEl> for El {
    fn from(field: FieldEl) -> Self {
        El::leaf(Widget::Field(field.0))
    }
}

/// Button under construction.
#[derive(Debug, Clone, PartialEq)]
pub struct ButtonEl(ButtonSpec);

pub fn button(id: impl Into<String>, label: impl Into<String>) -> ButtonEl {
    ButtonEl(ButtonSpec {
        id: id.into(),
        label: label.into(),
        style: ButtonStyle::Primary,
        enabled: true,
    })
}

impl ButtonEl {
    #[must_use]
    pub fn style(mut self, style: ButtonStyle) -> Self {
        self.0.style = style;
        self
    }

    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.0.enabled = enabled;
        self
    }
}

impl From<ButtonEl> for El {
    fn from(button: ButtonEl) -> Self {
        El::leaf(Widget::Button(button.0))
    }
}

/// Repeated rows under construction.
#[derive(Debug, Clone, PartialEq)]
pub struct ListEl {
    spec: ListSpec,
    rows: Vec<El>,
}

pub fn list(id: impl Into<String>, rows: Vec<El>) -> ListEl {
    ListEl {
        spec: ListSpec {
            id: id.into(),
            removable: false,
            add_options: None,
        },
        rows,
    }
}

impl ListEl {
    #[must_use]
    pub fn removable(mut self, removable: bool) -> Self {
        self.spec.removable = removable;
        self
    }

    #[must_use]
    pub fn add_options(mut self, options: Vec<PickOption>, placeholder: Option<String>) -> Self {
        self.spec.add_options = Some(PickList {
            options,
            selected: None,
            placeholder,
        });
        self
    }
}

impl From<ListEl> for El {
    fn from(list: ListEl) -> Self {
        El {
            widget: Widget::List(list.spec),
            children: list.rows,
        }
    }
}

/// `col![a, b, …]`: a column of anything convertible into [`El`].
#[macro_export]
macro_rules! col {
    ($($child:expr),* $(,)?) => {
        $crate::ui::column(::std::vec![$($crate::ui::El::from($child)),*])
    };
}

/// `row![a, b, …]`: a row of anything convertible into [`El`].
#[macro_export]
macro_rules! row {
    ($($child:expr),* $(,)?) => {
        $crate::ui::row(::std::vec![$($crate::ui::El::from($child)),*])
    };
}
