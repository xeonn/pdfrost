use std::fs;
use std::path::PathBuf;

use chrono::{TimeZone, Utc};
use pdfrost_engine::render;
use serde_json::Value;

fn read(name: &str) -> String {
    fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../jrxmls")
            .join(name),
    )
    .unwrap()
}

fn render_pair(template: &str, fixture: &str) -> pdfrost_engine::RenderOutput {
    let when = Utc.with_ymd_and_hms(2026, 10, 7, 9, 0, 0).unwrap();
    let document: Value = serde_json::from_str(&read(fixture)).unwrap();
    render(&read(template), &document, when).unwrap()
}

#[test]
fn sample_renders_the_category_charts() {
    let output = render_pair("Sample.jrxml", "fixtures/sample.json");
    assert!(output.pdf.starts_with(b"%PDF"));
    assert!(output.warnings.iter().all(|warning| warning.tag != "chart"));
    let text = pdf_extract::extract_text_from_mem(&output.pdf).unwrap();
    for label in [
        "Area",
        "Bar",
        "Line",
        "Stacked area",
        "Stacked bar",
        "Jan",
        "Units",
        "Returns",
    ] {
        assert!(text.contains(label), "missing {label} in {text:?}");
    }
}

#[test]
fn every_operational_template_prints_its_document_number_and_line_item() {
    let pairs = [
        ("Receive.jrxml", "fixtures/receive-empty.json", false),
        ("ShipmentOut.jrxml", "fixtures/shipment-out.json", true),
        (
            "StockAdjustment.jrxml",
            "fixtures/stock-adjustment.json",
            true,
        ),
        (
            "InterBranchTransfer.jrxml",
            "fixtures/inter-branch-transfer.json",
            true,
        ),
        (
            "InterLocatorTransfer.jrxml",
            "fixtures/inter-locator-transfer.json",
            true,
        ),
        (
            "InterWarehouseTransfer.jrxml",
            "fixtures/inter-warehouse-transfer.json",
            true,
        ),
    ];
    for (template, fixture, has_line) in pairs {
        let output = render_pair(template, fixture);
        assert!(output.pdf.starts_with(b"%PDF"), "{template}");
        let text = pdf_extract::extract_text_from_mem(&output.pdf).unwrap();
        assert!(
            text.contains("0001"),
            "{template} missing docno in {text:?}"
        );
        if has_line {
            assert!(
                text.contains("SKU-001"),
                "{template} missing a line item in {text:?}"
            );
        } else {
            assert!(
                !text.contains("SKU-001"),
                "{template} invented a line item in {text:?}"
            );
        }
    }
}
