use std::fs;
use std::path::PathBuf;

use chrono::{TimeZone, Utc};
use pdfrost_engine::{render, PlacementKind};
use serde_json::Value;

fn read_template(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../jrxmls")
        .join(name);
    fs::read_to_string(path).unwrap()
}

#[test]
fn sample_draws_title_headings_and_a_rectangle() {
    let when = Utc.with_ymd_and_hms(2026, 10, 7, 9, 0, 0).unwrap();
    let output = render(
        &read_template("Sample.jrxml"),
        &Value::Object(Default::default()),
        when,
    )
    .unwrap();
    assert!(output.pdf.starts_with(b"%PDF"));
    assert!(output.warnings.iter().any(|warning| warning.tag == "chart"));

    let text = pdf_extract::extract_text_from_mem(&output.pdf).unwrap();
    for label in [
        "Go binding to Jasper Report",
        "Company",
        "Contact Person",
        "ContactTitle",
        "Area",
        "Stacked bar",
    ] {
        assert!(text.contains(label), "missing {label} in {text:?}");
    }

    let title = output
        .placements
        .iter()
        .find(|placement| placement.text.as_deref() == Some("Go binding to Jasper Report"))
        .unwrap();
    // Title is the first flowing band, so its band offset is 0.
    assert_eq!(title.x, 20.0 + 0.0);
    assert_eq!(title.y, 20.0 + 0.0 + 10.0);
    assert_eq!(title.width, 555.0);
    assert_eq!(title.height, 30.0);

    let rectangle = output
        .placements
        .iter()
        .find(|placement| placement.kind == PlacementKind::Rectangle)
        .unwrap();
    let content_right = 595.0 - 20.0;
    let content_bottom = 842.0 - 20.0;
    assert!(rectangle.x >= 20.0);
    assert!(rectangle.y >= 20.0);
    assert!(rectangle.x + rectangle.width <= content_right);
    assert!(rectangle.y + rectangle.height <= content_bottom);
    assert_eq!(rectangle.stroke_width, None);
}
