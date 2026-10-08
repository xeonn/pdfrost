use std::fs;
use std::path::PathBuf;

use chrono::{TimeZone, Utc};
use pdfrost_engine::{parse, render, BandKind, Element, PlacementKind};
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
fn title_labels_use_the_margin_and_the_element_offset() {
    let when = Utc.with_ymd_and_hms(2026, 10, 7, 9, 0, 0).unwrap();
    let pairs = [
        ("Sample.jrxml", "fixtures/sample.json"),
        ("Receive.jrxml", "fixtures/receive-empty.json"),
        ("ShipmentOut.jrxml", "fixtures/shipment-out.json"),
        ("StockAdjustment.jrxml", "fixtures/stock-adjustment.json"),
        (
            "InterBranchTransfer.jrxml",
            "fixtures/inter-branch-transfer.json",
        ),
        (
            "InterLocatorTransfer.jrxml",
            "fixtures/inter-locator-transfer.json",
        ),
        (
            "InterWarehouseTransfer.jrxml",
            "fixtures/inter-warehouse-transfer.json",
        ),
    ];
    for (template, fixture) in pairs {
        let xml = read(template);
        let report = parse(&xml).unwrap();
        let document: Value = serde_json::from_str(&read(fixture)).unwrap();
        let output = render(&xml, &document, when).unwrap();
        let title = report
            .bands
            .iter()
            .find(|band| band.kind == BandKind::Title)
            .unwrap_or_else(|| panic!("{template} has no title"));
        let Element::StaticText(label) = title
            .elements
            .iter()
            .find(|element| matches!(element, Element::StaticText(_)))
            .unwrap_or_else(|| panic!("{template} title has no static text"))
        else {
            unreachable!();
        };
        let placed = output
            .placements
            .iter()
            .find(|placement| {
                placement.page == 1 && placement.text.as_deref() == Some(label.text.as_str())
            })
            .unwrap_or_else(|| panic!("{template} did not place {}", label.text));
        assert_eq!(placed.x, report.left_margin + label.element.x, "{template}");
        assert_eq!(placed.y, report.top_margin + label.element.y, "{template}");
    }
}

#[test]
fn shipment_columns_pens_and_the_continued_header_follow_the_template() {
    let when = Utc.with_ymd_and_hms(2026, 10, 7, 9, 0, 0).unwrap();
    let xml = read("ShipmentOut.jrxml");
    let report = parse(&xml).unwrap();
    let document: Value = serde_json::from_str(&read("fixtures/shipment-out.json")).unwrap();
    let output = render(&xml, &document, when).unwrap();

    let date = output
        .placements
        .iter()
        .find(|placement| placement.text.as_deref() == Some("Date"))
        .unwrap();
    assert_eq!(date.x, report.left_margin + 380.0);
    assert_eq!(date.y, report.top_margin);
    assert_eq!(date.stroke_width, Some(1.0));

    let header = output
        .placements
        .iter()
        .find(|placement| {
            placement.page == 1
                && placement.kind == PlacementKind::Rectangle
                && placement.width == 138.0
                && placement.height == 30.0
        })
        .unwrap();
    assert_eq!(header.x, report.left_margin);
    assert_eq!(header.y, report.top_margin + 260.0 + 20.0);
    assert_eq!(header.stroke_width, Some(0.5));

    let continued = output
        .placements
        .iter()
        .filter(|placement| {
            placement.page == 2
                && placement.kind == PlacementKind::Rectangle
                && placement.width == 138.0
                && placement.height == 30.0
        })
        .min_by(|left, right| left.y.total_cmp(&right.y))
        .unwrap();
    assert_eq!(continued.y, report.top_margin);
    assert_eq!(continued.stroke_width, Some(0.5));
}

#[test]
fn sample_reserves_a_box_for_the_chart() {
    let when = Utc.with_ymd_and_hms(2026, 10, 7, 9, 0, 0).unwrap();
    let document: Value = serde_json::from_str(&read("fixtures/sample.json")).unwrap();
    let report = pdfrost_engine::parse(&read("Sample.jrxml")).unwrap();
    let output = render(&read("Sample.jrxml"), &document, when).unwrap();
    assert!(output.warnings.iter().all(|warning| warning.tag != "chart"));
    let chart = output
        .placements
        .iter()
        .find(|placement| placement.kind == PlacementKind::Chart)
        .expect("chart");
    assert_eq!(chart.text.as_deref(), Some("areaChart"));
    assert_eq!(chart.page, 1);
    assert_eq!(chart.width, 270.0);
    assert_eq!(chart.height, 148.0);
    assert_eq!(chart.x, report.left_margin);
    let summary_top = report.top_margin + 70.0 + 35.0 + 30.0 + 20.0;
    assert_eq!(chart.y, summary_top);
    let kinds: Vec<_> = output
        .placements
        .iter()
        .filter(|placement| placement.kind == PlacementKind::Chart)
        .filter_map(|placement| placement.text.clone())
        .collect();
    assert_eq!(
        kinds,
        [
            "areaChart".to_string(),
            "barChart".to_string(),
            "lineChart".to_string(),
            "stackedAreaChart".to_string(),
            "stackedBarChart".to_string()
        ]
    );
}
