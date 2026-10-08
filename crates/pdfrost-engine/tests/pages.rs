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

#[test]
fn shipment_breaks_across_pages_and_repeats_the_column_header() {
    let when = Utc.with_ymd_and_hms(2026, 10, 7, 9, 0, 0).unwrap();
    let document: Value = serde_json::from_str(&read("fixtures/shipment-out.json")).unwrap();
    let output = render(&read("ShipmentOut.jrxml"), &document, when).unwrap();
    let pages = pdf_extract::extract_text_from_mem_by_pages(&output.pdf).unwrap();
    assert!(
        pages.len() > 1,
        "expected more than one page, got {}",
        pages.len()
    );

    let all = pages.join("\n");
    assert!(all.contains("SKU-001"), "missing a line item in {all:?}");
    assert!(
        all.contains("2026-10-07 09:00:00"),
        "missing pinned timestamp in {all:?}"
    );
    assert!(
        output
            .placements
            .iter()
            .any(|placement| placement.page == 1 && placement.text.as_deref() == Some("1")),
        "missing page number"
    );
    assert!(
        pages[1].contains("Item No"),
        "page 2 did not repeat the column header: {:?}",
        pages[1]
    );
}

#[test]
fn empty_receive_keeps_the_header_and_skips_detail_rows() {
    let when = Utc.with_ymd_and_hms(2026, 10, 7, 9, 0, 0).unwrap();
    let document: Value = serde_json::from_str(&read("fixtures/receive-empty.json")).unwrap();
    let output = render(&read("Receive.jrxml"), &document, when).unwrap();
    let text = pdf_extract::extract_text_from_mem(&output.pdf).unwrap();
    assert!(text.contains("Item No"), "missing table header in {text:?}");
    assert!(text.contains("0001"), "missing docno in {text:?}");
    assert!(!text.contains("SKU-001"), "invented a detail row: {text:?}");
}
