use std::fs;
use std::io::Cursor;
use std::path::PathBuf;

use chrono::{TimeZone, Utc};
use pdfrost_engine::{render, render_read};
use serde_json::json;

fn when() -> chrono::DateTime<chrono::Utc> {
    Utc.with_ymd_and_hms(2026, 10, 12, 9, 0, 0).unwrap()
}

fn template() -> String {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<jasperReport name="Records" pageWidth="200" pageHeight="100" columnWidth="180" leftMargin="10" rightMargin="10" topMargin="10" bottomMargin="10">
  <field name="sku" class="java.lang.String"/>
  <title>
    <band height="20" splitType="Stretch">
      <staticText>
        <reportElement x="0" y="0" width="40" height="16"/>
        <text><![CDATA[List]]></text>
      </staticText>
      <textField>
        <reportElement x="50" y="0" width="80" height="16"/>
        <textFieldExpression><![CDATA[$F{sku}]]></textFieldExpression>
      </textField>
    </band>
  </title>
  <detail>
    <band height="30" splitType="Stretch">
      <textField>
        <reportElement x="0" y="0" width="80" height="16"/>
        <textFieldExpression><![CDATA[$F{sku}]]></textFieldExpression>
      </textField>
    </band>
  </detail>
  <summary>
    <band height="16" splitType="Stretch">
      <staticText>
        <reportElement x="0" y="0" width="40" height="16"/>
        <text><![CDATA[End]]></text>
      </staticText>
      <textField>
        <reportElement x="50" y="0" width="80" height="16"/>
        <textFieldExpression><![CDATA[$F{sku}]]></textFieldExpression>
      </textField>
    </band>
  </summary>
</jasperReport>"#
        .to_string()
}

fn pages_of(pdf: &[u8]) -> Vec<String> {
    pdf_extract::extract_text_from_mem_by_pages(pdf).unwrap()
}

#[test]
fn a_root_array_repeats_detail_and_keeps_both_pages() {
    let document = json!([{ "sku": "BOLT" }, { "sku": "NUT" }, { "sku": "WASHER" }]);
    let output = render(&template(), &document, when()).unwrap();
    let pages = pages_of(&output.pdf);
    assert_eq!(pages.len(), 2, "{pages:?}");
    assert!(pages[0].contains("List"), "{pages:?}");
    assert!(pages[0].contains("BOLT"), "{pages:?}");
    assert!(pages[0].contains("NUT"), "{pages:?}");
    assert!(!pages[0].contains("WASHER"), "{pages:?}");
    assert!(pages[1].contains("WASHER"), "{pages:?}");
    assert!(pages[1].contains("End"), "{pages:?}");
    assert!(
        output.placements.iter().any(|placement| {
            placement.page == 1 && placement.y < 20.0 && placement.text.as_deref() == Some("BOLT")
        }),
        "title should show the first record: {:?}",
        output.placements
    );
    assert!(
        output.placements.iter().any(|placement| {
            placement.page == 2 && placement.y > 30.0 && placement.text.as_deref() == Some("WASHER")
        }),
        "summary should show the last record: {:?}",
        output.placements
    );
}

#[test]
fn an_object_still_fills_once() {
    let document = json!({ "sku": "Only" });
    let output = render(&template(), &document, when()).unwrap();
    let pages = pages_of(&output.pdf);
    assert_eq!(pages.len(), 1, "{pages:?}");
    let text = pages.join("\n");
    assert!(text.contains("Only"), "{text:?}");
    assert!(text.contains("End"), "{text:?}");
    assert!(!text.contains("NUT"), "{text:?}");
}

#[test]
fn render_read_matches_a_parsed_array() {
    let bytes = br#"[{"sku":"BOLT"},{"sku":"NUT"},{"sku":"WASHER"}]"#;
    let streamed = render_read(&template(), Cursor::new(&bytes[..]), when()).unwrap();
    let parsed = render(
        &template(),
        &json!([{ "sku": "BOLT" }, { "sku": "NUT" }, { "sku": "WASHER" }]),
        when(),
    )
    .unwrap();
    let streamed_pages = pages_of(&streamed.pdf);
    let parsed_pages = pages_of(&parsed.pdf);
    assert_eq!(streamed_pages, parsed_pages);
}

#[test]
fn an_empty_array_prints_the_title_and_skips_detail() {
    let output = render(&template(), &json!([]), when()).unwrap();
    let text = pdf_extract::extract_text_from_mem(&output.pdf).unwrap();
    assert!(text.contains("List"), "{text:?}");
    assert!(text.contains("End"), "{text:?}");
    assert!(!text.contains("BOLT"), "{text:?}");
}

#[test]
fn the_receipts_sample_spans_two_pages() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../jrxmls");
    let template = fs::read_to_string(root.join("Receipts.jrxml")).unwrap();
    let json = fs::read(root.join("Receipts.json")).unwrap();
    let output = render_read(&template, Cursor::new(json), when()).unwrap();
    assert!(output.pdf.starts_with(b"%PDF"));
    assert!(output.warnings.is_empty(), "{:?}", output.warnings);
    let text = pdf_extract::extract_text_from_mem(&output.pdf).unwrap();
    for label in [
        "Receipts",
        "Receipt",
        "Customer",
        "Amount",
        "RCP-1001",
        "Maria Anders",
        "RCP-1010",
        "Elizabeth Lincoln",
        "Last receipt",
    ] {
        assert!(text.contains(label), "missing {label} in {text:?}");
    }
    assert_eq!(
        output
            .placements
            .iter()
            .map(|placement| placement.page)
            .max(),
        Some(2)
    );
    assert!(output
        .placements
        .iter()
        .any(|placement| { placement.page == 1 && placement.text.as_deref() == Some("RCP-1001") }));
    assert!(output
        .placements
        .iter()
        .any(|placement| { placement.page == 2 && placement.text.as_deref() == Some("RCP-1010") }));
    assert!(output.placements.iter().any(|placement| {
        placement.page == 2 && placement.text.as_deref() == Some("Last receipt")
    }));
}

#[test]
fn render_read_rejects_text_that_is_not_json() {
    let error = render_read(&template(), Cursor::new(&b"not json"[..]), when()).unwrap_err();
    let message = error.to_string();
    assert!(
        message.contains("JSON") || message.contains("json"),
        "{message}"
    );
}
