//! Fill a parsed chart into the plot `pdfrost-charts` paints.
//!
//! Expressions are parsed by [`Expression`](crate::expr::Expression).
//! A chart prints a string literal, a number literal, or `$F{name}`.
//! Page number and `new java.util.Date()` stay text-field forms and warn here.

use pdfrost_charts::{CategoryChart, CategoryDataset, ChartKind};
use serde_json::Value;

use crate::expr::{self, Expression};
use crate::fill;
use crate::model::{Chart, Field, SubDataset};
use crate::Warning;

pub(crate) fn prepare(
    chart: &Chart,
    fields: &[Field],
    subdatasets: &[SubDataset],
    document: &Value,
    warnings: &mut Vec<Warning>,
) -> Option<CategoryChart> {
    let kind = chart_kind(&chart.kind)?;
    let fields = chart_fields(chart, fields, subdatasets);
    let mut dataset = CategoryDataset::new();
    for row in rows(chart, document) {
        for series in &chart.series {
            let Some(name) = label(
                &series.series_expression,
                fields,
                row,
                &chart.kind,
                warnings,
            ) else {
                continue;
            };
            let Some(category) = label(
                &series.category_expression,
                fields,
                row,
                &chart.kind,
                warnings,
            ) else {
                continue;
            };
            let Some(value) = number(&series.value_expression, fields, row, &chart.kind, warnings)
            else {
                continue;
            };
            dataset.add_value(name, category, value);
        }
    }
    if dataset.is_empty() {
        warnings.push(Warning {
            tag: "chart".to_string(),
            location: format!("{}/chart", chart.kind),
        });
    }
    Some(CategoryChart {
        kind,
        title: chart
            .title
            .as_deref()
            .and_then(|expression| label(expression, fields, document, &chart.kind, warnings)),
        subtitle: chart
            .subtitle
            .as_deref()
            .and_then(|expression| label(expression, fields, document, &chart.kind, warnings)),
        show_legend: chart.show_legend,
        dataset,
    })
}

fn chart_kind(kind: &str) -> Option<ChartKind> {
    Some(match kind {
        "areaChart" => ChartKind::Area,
        "barChart" => ChartKind::Bar,
        "lineChart" => ChartKind::Line,
        "stackedAreaChart" => ChartKind::StackedArea,
        "stackedBarChart" => ChartKind::StackedBar,
        _ => return None,
    })
}

fn chart_fields<'a>(
    chart: &Chart,
    fields: &'a [Field],
    subdatasets: &'a [SubDataset],
) -> &'a [Field] {
    chart
        .sub_dataset
        .as_deref()
        .and_then(|name| subdatasets.iter().find(|dataset| dataset.name == name))
        .map(|dataset| dataset.fields.as_slice())
        .unwrap_or(fields)
}

fn rows<'a>(chart: &Chart, document: &'a Value) -> Vec<&'a Value> {
    let Some(name) = chart.data_source.as_deref() else {
        return vec![document];
    };
    match document.get(name) {
        Some(Value::Array(items)) => items.iter().collect(),
        _ => Vec::new(),
    }
}

fn label(
    expression: &str,
    fields: &[Field],
    data: &Value,
    location: &str,
    warnings: &mut Vec<Warning>,
) -> Option<String> {
    match eval(expression, fields, data, location, warnings)? {
        ChartValue::Text(text) => Some(text),
        ChartValue::Number(value) => Some(number_text(value)),
    }
}

fn number(
    expression: &str,
    fields: &[Field],
    data: &Value,
    location: &str,
    warnings: &mut Vec<Warning>,
) -> Option<f64> {
    match eval(expression, fields, data, location, warnings)? {
        ChartValue::Number(value) => Some(value),
        ChartValue::Text(text) => match text.parse::<f64>() {
            Ok(value) => Some(value),
            Err(_) => {
                warnings.push(Warning {
                    tag: expression.trim().to_string(),
                    location: location.to_string(),
                });
                None
            }
        },
    }
}

enum ChartValue {
    Text(String),
    Number(f64),
}

fn eval(
    expression: &str,
    fields: &[Field],
    data: &Value,
    location: &str,
    warnings: &mut Vec<Warning>,
) -> Option<ChartValue> {
    match Expression::parse(expression) {
        Expression::Text(text) => Some(ChartValue::Text(text)),
        Expression::Number(value) => Some(ChartValue::Number(value)),
        Expression::Field(name) => {
            let filled = fill::field(&name, fields, data, location);
            if let Some(warning) = filled.warning {
                warnings.push(warning);
                return None;
            }
            if filled.text.is_empty() {
                return None;
            }
            if let Some(value) = expr::number_literal(&filled.text) {
                return Some(ChartValue::Number(value));
            }
            Some(ChartValue::Text(filled.text))
        }
        Expression::Empty => None,
        Expression::PageNumber | Expression::Now | Expression::Unknown(_) => {
            warnings.push(Warning {
                tag: expression.trim().to_string(),
                location: location.to_string(),
            });
            None
        }
    }
}

fn number_text(value: f64) -> String {
    if value.is_finite() && value.fract() == 0.0 && value.abs() < i64::MAX as f64 {
        format!("{}", value as i64)
    } else {
        value.to_string()
    }
}
