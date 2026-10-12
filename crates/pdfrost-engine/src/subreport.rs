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
    root: &Value,
    warnings: &mut Vec<Warning>,
) -> Option<String> {
    match Expression::parse(expression) {
        Expression::Text(text) => Some(text),
        Expression::Field(name) => {
            let filled = fill::field(&name, fields, data, root, "subreport");
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

/// Records for one subreport fill. The values stay in the caller's document.
///
/// No path fills the current object once. `.` is the document itself. A dotted
/// path walks objects, and a numeric segment indexes an array. An array becomes
/// one record per element. A missing path is an error.
pub(crate) fn records<'a>(document: &'a Value, source: Option<&str>) -> Result<Records<'a>, ()> {
    let Some(path) = source.map(str::trim).filter(|path| !path.is_empty()) else {
        return Ok(Records::One(document));
    };
    let Some(value) = at_path(document, path) else {
        return Err(());
    };
    match value {
        Value::Array(items) => Ok(Records::Many(items)),
        other => Ok(Records::One(other)),
    }
}

/// The main record set. A query path that exists in the document replaces it.
/// A missing path leaves the document unchanged.
pub(crate) fn main_document<'a>(document: &'a Value, query: Option<&str>) -> &'a Value {
    let Some(query) = query.map(str::trim).filter(|query| !query.is_empty()) else {
        return document;
    };
    at_path(document, query).unwrap_or(document)
}

/// `source` is read from `record`. When that path is missing, it is read from `root`.
pub(crate) fn records_near<'a>(
    record: &'a Value,
    root: &'a Value,
    source: Option<&str>,
) -> Result<Records<'a>, ()> {
    match records(record, source) {
        Ok(found) => Ok(found),
        Err(()) if !std::ptr::eq(record, root) => records(root, source),
        Err(()) => Err(()),
    }
}

/// A borrowed view of the records a subreport fills.
#[derive(Clone, Copy)]
pub(crate) enum Records<'a> {
    One(&'a Value),
    Many(&'a [Value]),
}

impl<'a> Records<'a> {
    pub(crate) fn is_empty(self) -> bool {
        matches!(self, Records::Many(items) if items.is_empty())
    }

    pub(crate) fn len(self) -> usize {
        match self {
            Records::One(_) => 1,
            Records::Many(items) => items.len(),
        }
    }

    pub(crate) fn get(self, index: usize) -> Option<&'a Value> {
        match self {
            Records::One(value) if index == 0 => Some(value),
            Records::One(_) => None,
            Records::Many(items) => items.get(index),
        }
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
