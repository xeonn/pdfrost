//! Resolve `$F{name}` against the one JSON document.
//!
//! A missing path or a JSON null prints blank and records a warning. The word `null` is never printed.

use chrono::{DateTime, Datelike, Timelike, Utc};
use serde_json::Value;

use crate::expr::Expression;
use crate::model::Field;
use crate::Warning;

pub(crate) struct Filled {
    pub text: String,
    pub warning: Option<Warning>,
}

/// Evaluate one text-field expression. Page number and the pinned timestamp do not warn.
/// Quoted strings and number literals stay blank here; charts are the caller that prints them.
pub(crate) fn eval(
    expression: &str,
    fields: &[Field],
    data: &Value,
    location: &str,
    page: u32,
    rendered_at: DateTime<Utc>,
) -> Filled {
    match Expression::parse(expression) {
        Expression::PageNumber => Filled {
            text: page.to_string(),
            warning: None,
        },
        Expression::Now => Filled {
            text: format_utc(rendered_at),
            warning: None,
        },
        Expression::Field(name) => field(&name, fields, data, location),
        Expression::Empty
        | Expression::Text(_)
        | Expression::Number(_)
        | Expression::Unknown(_) => Filled {
            text: String::new(),
            warning: None,
        },
    }
}

pub(crate) fn field(name: &str, fields: &[Field], document: &Value, location: &str) -> Filled {
    let Some(field) = fields.iter().find(|field| field.name == name) else {
        return blank(name, location);
    };
    let path = field.json_path.as_deref().unwrap_or(name);
    let Some(value) = lookup(document, path) else {
        return blank(name, location);
    };
    match printable(field, value) {
        Some(text) => Filled {
            text,
            warning: None,
        },
        None => blank(name, location),
    }
}

fn format_utc(instant: DateTime<Utc>) -> String {
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        instant.year(),
        instant.month(),
        instant.day(),
        instant.hour(),
        instant.minute(),
        instant.second()
    )
}

fn blank(name: &str, location: &str) -> Filled {
    Filled {
        text: String::new(),
        warning: Some(Warning {
            tag: name.to_string(),
            location: location.to_string(),
        }),
    }
}

fn lookup<'a>(document: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = document;
    for part in path.split('.') {
        current = current.get(part)?;
    }
    Some(current)
}

fn printable(field: &Field, value: &Value) -> Option<String> {
    if value.is_null() {
        return None;
    }
    if field.is_integer() {
        return integer_digits(value);
    }
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(flag) => Some(flag.to_string()),
        _ => None,
    }
}

fn integer_digits(value: &Value) -> Option<String> {
    match value {
        Value::Number(number) => number
            .as_i64()
            .map(|number| number.to_string())
            .or_else(|| number.as_u64().map(|number| number.to_string())),
        Value::String(text) => {
            let text = text.trim();
            text.parse::<i64>().ok().map(|number| number.to_string())
        }
        _ => None,
    }
}
