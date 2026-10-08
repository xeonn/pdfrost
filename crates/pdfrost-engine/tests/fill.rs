use std::fs;
use std::path::PathBuf;

use chrono::{TimeZone, Utc};
use pdfrost_engine::render;
use serde_json::Value;

fn read(path: &str) -> String {
    fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path),
    )
    .unwrap()
}

fn render_pair(template: &str, fixture: &str) -> pdfrost_engine::RenderOutput {
    let when = Utc.with_ymd_and_hms(2026, 10, 7, 9, 0, 0).unwrap();
    let document: Value = serde_json::from_str(&read(fixture)).unwrap();
    render(&read(template), &document, when).unwrap()
}

fn text_of(output: &pdfrost_engine::RenderOutput) -> String {
    pdf_extract::extract_text_from_mem(&output.pdf).unwrap()
}

#[test]
fn sample_fills_company_and_contact() {
    let output = render_pair("jrxmls/Sample.jrxml", "jrxmls/fixtures/sample.json");
    let text = text_of(&output);
    for value in ["Northwind Traders", "Maria Anders", "Sales Representative"] {
        assert!(text.contains(value), "missing {value} in {text:?}");
    }
}

#[test]
fn shipment_fills_docno_nested_path_and_integer() {
    let output = render_pair(
        "jrxmls/ShipmentOut.jrxml",
        "jrxmls/fixtures/shipment-out.json",
    );
    let text = text_of(&output);
    assert!(text.contains("0001"), "missing docno in {text:?}");
    assert!(
        text.contains("2026-10-01T08:00:00Z"),
        "missing audit.created in {text:?}"
    );
    assert!(text.contains("1001"), "missing integer invno in {text:?}");
}

#[test]
fn missing_docno_warns_and_does_not_print_null() {
    let when = Utc.with_ymd_and_hms(2026, 10, 7, 9, 0, 0).unwrap();
    let mut document: Value =
        serde_json::from_str(&read("jrxmls/fixtures/shipment-out.json")).unwrap();
    document.as_object_mut().unwrap().remove("docno");

    let output = render(&read("jrxmls/ShipmentOut.jrxml"), &document, when).unwrap();
    let text = text_of(&output);
    assert!(
        output.warnings.iter().any(|warning| warning.tag == "docno"),
        "{:?}",
        output.warnings
    );
    assert!(!text.contains("null"), "{text:?}");
    assert!(!text.contains("0001"), "{text:?}");
}
