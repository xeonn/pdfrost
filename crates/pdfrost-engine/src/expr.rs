//! The expression grammar text fields and charts share.
//!
//! One parse produces the tree. Each caller interprets the forms it accepts.
//! Text fields print a field, the page number, or the pinned timestamp.
//! Charts also accept a quoted string or a number literal.

/// A whole expression, after trimming. The source text is not stored on the report.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Expression {
    Empty,
    Field(String),
    PageNumber,
    Now,
    Text(String),
    Number(f64),
    Unknown(String),
}

impl Expression {
    pub(crate) fn parse(source: &str) -> Self {
        let source = source.trim();
        if source.is_empty() {
            return Expression::Empty;
        }
        if source == "$V{PAGE_NUMBER}" {
            return Expression::PageNumber;
        }
        if source == "new java.util.Date()" {
            return Expression::Now;
        }
        if let Some(name) = field_name(source) {
            return Expression::Field(name.to_string());
        }
        if let Some(text) = quoted(source) {
            return Expression::Text(text.to_string());
        }
        if let Some(value) = number_literal(source) {
            return Expression::Number(value);
        }
        Expression::Unknown(source.to_string())
    }

    /// Forms a text field may print. Anything else stays blank and does not warn at parse time.
    pub(crate) fn is_text_field(&self) -> bool {
        matches!(
            self,
            Expression::Field(_) | Expression::PageNumber | Expression::Now
        )
    }
}

fn field_name(expression: &str) -> Option<&str> {
    let name = expression.strip_prefix("$F{")?.strip_suffix('}')?;
    if name.is_empty() || name.contains('{') || name.contains('}') {
        None
    } else {
        Some(name)
    }
}

fn quoted(expression: &str) -> Option<&str> {
    expression
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
}

pub(crate) fn number_literal(expression: &str) -> Option<f64> {
    let expression = expression.trim();
    if expression.is_empty()
        || !expression
            .chars()
            .all(|ch| ch.is_ascii_digit() || matches!(ch, '.' | '-' | '+'))
    {
        return None;
    }
    expression.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_fields_and_charts_share_one_grammar() {
        assert_eq!(Expression::parse("  ").is_text_field(), false);
        assert!(matches!(Expression::parse("  "), Expression::Empty));
        assert!(Expression::parse("$F{qty}").is_text_field());
        assert!(Expression::parse("$V{PAGE_NUMBER}").is_text_field());
        assert!(Expression::parse("new java.util.Date()").is_text_field());
        assert_eq!(
            Expression::parse("\"Hello\""),
            Expression::Text("Hello".to_string())
        );
        assert_eq!(Expression::parse("12.5"), Expression::Number(12.5));
        assert!(matches!(
            Expression::parse("$P{report}"),
            Expression::Unknown(_)
        ));
        assert!(!Expression::parse("\"Hello\"").is_text_field());
    }
}
