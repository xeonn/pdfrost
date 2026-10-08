use std::fs;
use std::path::PathBuf;

use chrono::{TimeZone, Utc};
use pdfrost_engine::{render, Error, DEFAULT_SANS_BOLD, DEFAULT_SANS_REGULAR};
use serde_json::Value;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../jrxmls/fixtures")
}

fn read_json(name: &str) -> Value {
    let path = fixtures_dir().join(name);
    let text =
        fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|err| panic!("parse {}: {err}", path.display()))
}

#[test]
fn render_rejects_a_page_without_size() {
    let when = Utc.with_ymd_and_hms(2026, 10, 7, 9, 0, 0).unwrap();
    let err = render("<jasperReport/>", &Value::Null, when).unwrap_err();
    assert_eq!(err, Error::PageSize);
}

#[test]
fn default_sans_files_are_truetype() {
    assert!(DEFAULT_SANS_REGULAR.starts_with(&[0x00, 0x01, 0x00, 0x00]));
    assert!(DEFAULT_SANS_BOLD.starts_with(&[0x00, 0x01, 0x00, 0x00]));
}

#[test]
fn fixtures_match_template_field_paths() {
    let sample = read_json("sample.json");
    assert_eq!(sample["CompanyName"], "Northwind Traders");
    assert_eq!(sample["ContactName"], "Maria Anders");
    assert_eq!(sample["ContactTitle"], "Sales Representative");
    assert!(sample["PostalCode"].is_number());

    let shipment = read_json("shipment-out.json");
    let items = shipment["itemsList"].as_array().expect("itemsList");
    assert!(items.len() >= 25, "shipment items should cross a page");
    assert!(items[0]["qty"].is_number());
    assert!(items[0]["product"]["id"].is_string());
    assert!(shipment["invno"].is_number());
    assert!(shipment["audit"]["created"].is_string());

    let empty = read_json("receive-empty.json");
    assert_eq!(
        empty["itemsList"].as_array().map(|rows| rows.len()),
        Some(0)
    );
    assert!(empty["locator"]["name"].is_string());

    for name in [
        "stock-adjustment.json",
        "inter-branch-transfer.json",
        "inter-locator-transfer.json",
        "inter-warehouse-transfer.json",
    ] {
        let doc = read_json(name);
        let rows = doc["itemsList"]
            .as_array()
            .unwrap_or_else(|| panic!("{name} itemsList"));
        assert!(!rows.is_empty(), "{name}");
        assert!(rows[0]["qty"].is_number(), "{name} qty");
    }
}
