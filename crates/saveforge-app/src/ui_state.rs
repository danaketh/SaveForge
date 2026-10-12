//! Host-side state the plugin never sees (spec §4.5): the raw text of number
//! fields while the user types, and the pending choice of each list's
//! "Add" pick-list.

use std::collections::{HashMap, HashSet};

use saveforge_plugin_api::ui::{Input, NumberInput, UiTree, Widget};

#[derive(Debug, Clone, Default)]
pub struct UiState {
    numbers: HashMap<String, NumberBuffer>,
    list_choices: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq)]
struct NumberBuffer {
    text: String,
    /// The plugin's value when this buffer was written. A different value in a
    /// later tree means the plugin changed it, so the buffer is discarded.
    plugin_value: f64,
    error: Option<String>,
}

/// The plugin's value as the field shows it, with `decimals` places.
pub fn format_number(input: &NumberInput) -> String {
    format!("{:.*}", usize::from(input.decimals), input.value)
}

/// Parses user text against the field's rules. `Err` is the message shown
/// under the field.
pub fn parse_number(text: &str, input: &NumberInput) -> Result<f64, String> {
    let trimmed = text.trim();
    let value = trimmed
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .ok_or_else(|| "Enter a number".to_owned())?;
    let fraction_digits = trimmed.split_once('.').map_or(0, |(_, f)| f.len());
    if fraction_digits > usize::from(input.decimals) {
        return Err(if input.decimals == 0 {
            "Whole numbers only".to_owned()
        } else {
            format!("At most {} decimal places", input.decimals)
        });
    }
    if let Some(min) = input.min
        && value < min
    {
        return Err(format!("Must be at least {min}"));
    }
    if let Some(max) = input.max
        && value > max
    {
        return Err(format!("Must be at most {max}"));
    }
    Ok(value)
}

/// The number input spec of field `id` in `tree`, if any.
pub fn find_number_input<'a>(tree: &'a UiTree, id: &str) -> Option<&'a NumberInput> {
    tree.nodes.iter().find_map(|node| match &node.widget {
        Widget::Field(field) if field.id == id => match &field.input {
            Input::Number(input) => Some(input),
            Input::Text(_) | Input::Checkbox(_) | Input::Pick(_) => None,
        },
        _ => None,
    })
}

impl UiState {
    /// Text and error to show for number field `id`.
    pub fn number_display(&self, id: &str, input: &NumberInput) -> (String, Option<String>) {
        match self.numbers.get(id) {
            Some(buffer) if buffer.plugin_value.to_bits() == input.value.to_bits() => {
                (buffer.text.clone(), buffer.error.clone())
            }
            _ => (format_number(input), None),
        }
    }

    /// Records what the user typed into number field `id`. Returns the value
    /// to send to the plugin when the text is valid.
    pub fn edit_number(&mut self, id: &str, text: String, input: &NumberInput) -> Option<f64> {
        let (plugin_value, error, send) = match parse_number(&text, input) {
            Ok(value) => (value, None, Some(value)),
            Err(error) => (input.value, Some(error), None),
        };
        self.numbers.insert(
            id.to_owned(),
            NumberBuffer {
                text,
                plugin_value,
                error,
            },
        );
        send
    }

    pub fn list_choice(&self, id: &str) -> Option<&str> {
        self.list_choices.get(id).map(String::as_str)
    }

    pub fn set_list_choice(&mut self, id: &str, value: String) {
        self.list_choices.insert(id.to_owned(), value);
    }

    pub fn clear_list_choice(&mut self, id: &str) {
        self.list_choices.remove(id);
    }

    /// Drops state for widgets that are no longer in `tree`.
    pub fn retain(&mut self, tree: &UiTree) {
        let mut numbers = HashSet::new();
        let mut lists = HashSet::new();
        for node in &tree.nodes {
            match &node.widget {
                Widget::Field(field) if matches!(field.input, Input::Number(_)) => {
                    numbers.insert(field.id.as_str());
                }
                Widget::List(list) => {
                    lists.insert(list.id.as_str());
                }
                _ => {}
            }
        }
        self.numbers.retain(|id, _| numbers.contains(id.as_str()));
        self.list_choices
            .retain(|id, _| lists.contains(id.as_str()));
    }
}
