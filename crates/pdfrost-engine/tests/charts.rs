use chrono::{TimeZone, Utc};
use pdfrost_engine::{parse, render, write_report, PlacementKind};
use serde_json::json;

fn render_xml(template: &str, document: serde_json::Value) -> pdfrost_engine::RenderOutput {
    let when = Utc.with_ymd_and_hms(2026, 10, 8, 9, 0, 0).unwrap();
    render(template, &document, when).unwrap()
}

fn chart_report(tag: &str, body: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<jasperReport name="Chart" pageWidth="300" pageHeight="300" columnWidth="260" leftMargin="20" rightMargin="20" topMargin="20" bottomMargin="20">
  <field name="sku" class="java.lang.String"/>
  <field name="qty" class="java.lang.Integer"/>
  <summary>
    <band height="200" splitType="Stretch">
      <{tag}>
        <chart>
          <reportElement x="0" y="0" width="240" height="180"/>
        </chart>
        {body}
      </{tag}>
    </band>
  </summary>
</jasperReport>"#
    )
}

#[test]
fn sample_chart_series_round_trips_through_the_writer() {
    let report = parse(include_str!("../../../jrxmls/Sample.jrxml")).unwrap();
    let again = parse(&write_report(&report)).unwrap();
    let original = report
        .bands
        .iter()
        .find(|band| band.kind == pdfrost_engine::BandKind::Summary)
        .unwrap();
    let written = again
        .bands
        .iter()
        .find(|band| band.kind == pdfrost_engine::BandKind::Summary)
        .unwrap();
    let pdfrost_engine::Element::Chart(before) = &original.elements[2] else {
        panic!("sample summary chart");
    };
    let pdfrost_engine::Element::Chart(after) = &written.elements[2] else {
        panic!("written summary chart");
    };
    assert_eq!(after.kind, before.kind);
    assert_eq!(after.element, before.element);
    assert_eq!(after.series, before.series);
    assert_eq!(after.evaluation_time, before.evaluation_time);
    assert_eq!(after.show_legend, before.show_legend);
}

#[test]
fn an_area_of_two_categories_names_both_on_the_page() {
    let template = chart_report(
        "areaChart",
        r#"<categoryDataset>
          <categorySeries>
            <seriesExpression><![CDATA["A"]]></seriesExpression>
            <categoryExpression><![CDATA["Jan"]]></categoryExpression>
            <valueExpression><![CDATA[1]]></valueExpression>
          </categorySeries>
          <categorySeries>
            <seriesExpression><![CDATA["A"]]></seriesExpression>
            <categoryExpression><![CDATA["Feb"]]></categoryExpression>
            <valueExpression><![CDATA[3]]></valueExpression>
          </categorySeries>
        </categoryDataset>"#,
    );
    let output = render_xml(&template, json!({}));
    assert!(output.warnings.iter().all(|warning| warning.tag != "chart"));
    let text = pdf_extract::extract_text_from_mem(&output.pdf).unwrap();
    assert!(text.contains("Jan"), "{text}");
    assert!(text.contains("Feb"), "{text}");
    assert!(text.contains('A'), "{text}");
}

#[test]
fn a_subdataset_bar_plots_one_point_per_row() {
    let template = chart_report(
        "barChart",
        r#"<categoryDataset>
          <dataset>
            <datasetRun>
              <dataSourceExpression><![CDATA[subDataSource("itemsList")]]></dataSourceExpression>
            </datasetRun>
          </dataset>
          <categorySeries>
            <seriesExpression><![CDATA["Qty"]]></seriesExpression>
            <categoryExpression><![CDATA[$F{sku}]]></categoryExpression>
            <valueExpression><![CDATA[$F{qty}]]></valueExpression>
          </categorySeries>
        </categoryDataset>"#,
    );
    let output = render_xml(
        &template,
        json!({"itemsList": [{"sku": "Bolt", "qty": 1}, {"sku": "Nut", "qty": 3}]}),
    );
    assert!(output.warnings.iter().all(|warning| warning.tag != "chart"));
    let text = pdf_extract::extract_text_from_mem(&output.pdf).unwrap();
    assert!(text.contains("Bolt"), "{text}");
    assert!(text.contains("Nut"), "{text}");
    assert!(text.contains("Qty"), "{text}");
}

#[test]
fn line_and_stacked_charts_draw_their_series_names() {
    for tag in ["lineChart", "stackedBarChart", "stackedAreaChart"] {
        let template = chart_report(
            tag,
            r#"<categoryDataset>
              <categorySeries>
                <seriesExpression><![CDATA["A"]]></seriesExpression>
                <categoryExpression><![CDATA["Jan"]]></categoryExpression>
                <valueExpression><![CDATA[1]]></valueExpression>
              </categorySeries>
              <categorySeries>
                <seriesExpression><![CDATA["B"]]></seriesExpression>
                <categoryExpression><![CDATA["Jan"]]></categoryExpression>
                <valueExpression><![CDATA[2]]></valueExpression>
              </categorySeries>
            </categoryDataset>"#,
        );
        let output = render_xml(&template, json!({}));
        assert!(
            output.warnings.iter().all(|warning| warning.tag != "chart"),
            "{tag}: {:?}",
            output.warnings
        );
        let text = pdf_extract::extract_text_from_mem(&output.pdf).unwrap();
        assert!(text.contains('A') && text.contains('B'), "{tag}: {text}");
        assert_eq!(
            output
                .placements
                .iter()
                .find(|placement| placement.kind == PlacementKind::Chart)
                .unwrap()
                .text
                .as_deref(),
            Some(tag)
        );
    }
}

#[test]
fn a_pie_chart_stays_a_placeholder_and_a_warning() {
    let template = chart_report("pieChart", "<pieDataset/>");
    let output = render_xml(&template, json!({}));
    let warning = output
        .warnings
        .iter()
        .find(|warning| warning.tag == "chart")
        .expect("pie chart warning");
    assert!(warning.location.contains("pieChart"));
    let chart = output
        .placements
        .iter()
        .find(|placement| placement.kind == PlacementKind::Chart)
        .unwrap();
    assert_eq!(chart.text.as_deref(), Some("Chart"));
}
