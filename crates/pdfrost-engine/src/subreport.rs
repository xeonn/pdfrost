//! Resolve a subreport file and the JSON records it fills.
//!
//! The other file is a `.jrxml` path. Its records come from the same JSON
//! document: the current object, or the array named by `subDataSource("…")`.

use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::expr::Expression;
use crate::fill;
use crate::model::Field;
use crate::Warning;

pub(crate) fn expression_path(
    expression: &str,
    fields: &[Field],
    data: &Value,
    warnings: &mut Vec<Warning>,
) -> Option<String> {
    match Expression::parse(expression) {
        Expression::Text(text) => Some(text),
        Expression::Field(name) => {
            let filled = fill::field(&name, fields, data, "subreport");
            if let Some(warning) = filled.warning {
                warnings.push(warning);
            }
            if filled.text.is_empty() {
                None
            } else {
                Some(filled.text)
            }
        }
        Expression::Empty => None,
        Expression::Unknown(text) => {
            warnings.push(Warning {
                tag: text,
                location: "subreport".to_string(),
            });
            None
        }
        _ => {
            warnings.push(Warning {
                tag: expression.to_string(),
                location: "subreport".to_string(),
            });
            None
        }
    }
}

/// Find the `.jrxml`. A `.jasper` path uses the `.jrxml` with the same stem.
pub(crate) fn locate(
    source: &str,
    asset_dir: Option<&Path>,
    warnings: &mut Vec<Warning>,
) -> Option<PathBuf> {
    let source = source.trim();
    if source.is_empty() || source.contains('\0') {
        warnings.push(Warning {
            tag: "subreport".to_string(),
            location: "subreport".to_string(),
        });
        return None;
    }
    let renamed = source
        .strip_suffix(".jasper")
        .map(|stem| format!("{stem}.jrxml"));
    let name = renamed.as_deref().unwrap_or(source);
    let path = Path::new(name);
    let mut candidates = Vec::new();
    if path.is_absolute() {
        candidates.push(path.to_path_buf());
    } else if let Some(dir) = asset_dir {
        candidates.push(dir.join(path));
    }
    if !path.is_absolute() {
        candidates.push(path.to_path_buf());
    }
    let found = candidates.into_iter().find(|candidate| candidate.is_file());
    if found.is_some() && renamed.is_some() {
        warnings.push(Warning {
            tag: format!("subreport {source}"),
            location: "subreport".to_string(),
        });
    }
    if found.is_none() {
        warnings.push(Warning {
            tag: format!("subreport {source}"),
            location: "subreport".to_string(),
        });
    }
    found
}

/// Records for one subreport fill.
///
/// No path fills the current object once. `.` is the document itself. A dotted
/// path walks objects, and a numeric segment indexes an array. An array becomes
/// one record per element. A missing path is an error.
pub(crate) fn records(document: &Value, source: Option<&str>) -> Result<Vec<Value>, ()> {
    let Some(path) = source.map(str::trim).filter(|path| !path.is_empty()) else {
        return Ok(vec![document.clone()]);
    };
    let Some(value) = at_path(document, path) else {
        return Err(());
    };
    match value {
        Value::Array(items) => Ok(items.clone()),
        other => Ok(vec![other.clone()]),
    }
}

fn at_path<'a>(document: &'a Value, path: &str) -> Option<&'a Value> {
    if path == "." {
        return Some(document);
    }
    let mut current = document;
    for part in path.split('.') {
        if part.is_empty() {
            return None;
        }
        current = match current {
            Value::Array(items) => {
                let index: usize = part.parse().ok()?;
                items.get(index)?
            }
            other => other.get(part)?,
        };
    }
    Some(current)
}
