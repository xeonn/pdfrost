use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use chrono::{TimeZone, Utc};
use pdfrost_engine::{parse, render_with_assets, write_report, Element};
use serde_json::json;

static TEMP: AtomicU64 = AtomicU64::new(0);

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let id = TEMP.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("pdfrost-subreport-{}-{id}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn write(&self, name: &str, contents: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, contents).unwrap();
        path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn render_in(
    dir: &TempDir,
    template: &str,
    document: serde_json::Value,
) -> pdfrost_engine::RenderOutput {
    let when = Utc.with_ymd_and_hms(2026, 10, 12, 0, 0, 0).unwrap();
    render_with_assets(template, &document, when, Some(&dir.0)).unwrap()
}

fn texts(output: &pdfrost_engine::RenderOutput) -> Vec<String> {
    output
        .placements
        .iter()
        .filter_map(|placement| placement.text.clone())
        .collect()
}

const CHILD: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<jasperReport name="Child" pageWidth="200" pageHeight="400" columnWidth="200" leftMargin="0" rightMargin="0" topMargin="0" bottomMargin="0">
  <field name="company" class="java.lang.String"/>
  <field name="sku" class="java.lang.String"/>
  <detail>
    <band height="20">
      <textField>
        <reportElement x="0" y="0" width="80" height="16"/>
        <textFieldExpression><![CDATA[$F{company}]]></textFieldExpression>
      </textField>
      <textField>
        <reportElement x="80" y="0" width="80" height="16"/>
        <textFieldExpression><![CDATA[$F{sku}]]></textFieldExpression>
      </textField>
    </band>
  </detail>
</jasperReport>"#;

fn master(expression: &str, source: &str) -> String {
    let source = if source.is_empty() {
        String::new()
    } else {
        format!("<dataSourceExpression>subDataSource(\"{source}\")</dataSourceExpression>")
    };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<jasperReport name="Master" pageWidth="300" pageHeight="400" columnWidth="260" leftMargin="20" rightMargin="20" topMargin="20" bottomMargin="20">
  <detail>
    <band height="40">
      <staticText>
        <reportElement x="0" y="0" width="80" height="12"/>
        <text><![CDATA[Master]]></text>
      </staticText>
      <subreport>
        <reportElement x="0" y="16" width="200" height="20"/>
        {source}
        <subreportExpression><![CDATA[{expression}]]></subreportExpression>
      </subreport>
    </band>
  </detail>
</jasperReport>"#
    )
}

#[test]
fn the_current_object_fills_the_subreport_once() {
    let dir = TempDir::new();
    dir.write("header.jrxml", CHILD);
    let output = render_in(
        &dir,
        &master("\"header.jrxml\"", ""),
        json!({"company": "North", "sku": "Bolt"}),
    );
    let text = texts(&output);
    assert!(text.iter().any(|item| item == "Master"), "{text:?}");
    assert!(text.iter().any(|item| item == "North"), "{text:?}");
    assert!(text.iter().any(|item| item == "Bolt"), "{text:?}");
    assert!(output
        .warnings
        .iter()
        .all(|warning| !warning.tag.starts_with("subreport")));
}

#[test]
fn an_array_path_repeats_the_detail_band() {
    let dir = TempDir::new();
    dir.write("lines.jrxml", CHILD);
    let output = render_in(
        &dir,
        &master("\"lines.jrxml\"", "customer.orders"),
        json!({
            "customer": {
                "orders": [
                    {"sku": "Bolt"},
                    {"sku": "Nut"}
                ]
            }
        }),
    );
    let text = texts(&output);
    assert!(text.iter().any(|item| item == "Bolt"), "{text:?}");
    assert!(text.iter().any(|item| item == "Nut"), "{text:?}");
}

#[test]
fn a_table_inside_the_subreport_reads_the_row() {
    let dir = TempDir::new();
    dir.write(
        "order.jrxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<jasperReport name="Order" pageWidth="200" pageHeight="400" columnWidth="200" leftMargin="0" rightMargin="0" topMargin="0" bottomMargin="0">
  <subDataset name="Lines">
    <field name="sku" class="java.lang.String"/>
  </subDataset>
  <detail>
    <band height="40">
      <componentElement>
        <reportElement x="0" y="0" width="80" height="20"/>
        <table>
          <datasetRun subDataset="Lines">
            <dataSourceExpression>subDataSource("lines")</dataSourceExpression>
          </datasetRun>
          <column width="80">
            <detailCell height="14">
              <textField>
                <reportElement x="0" y="0" width="80" height="14"/>
                <textFieldExpression><![CDATA[$F{sku}]]></textFieldExpression>
              </textField>
            </detailCell>
          </column>
        </table>
      </componentElement>
    </band>
  </detail>
</jasperReport>"#,
    );
    let output = render_in(
        &dir,
        &master("\"order.jrxml\"", "orders"),
        json!({
            "orders": [
                {"lines": [{"sku": "Bolt"}]},
                {"lines": [{"sku": "Nut"}]}
            ]
        }),
    );
    let text = texts(&output);
    assert!(text.iter().any(|item| item == "Bolt"), "{text:?}");
    assert!(text.iter().any(|item| item == "Nut"), "{text:?}");
}

#[test]
fn a_jasper_path_reads_the_jrxml_and_warns() {
    let dir = TempDir::new();
    dir.write("header.jrxml", CHILD);
    let output = render_in(
        &dir,
        &master("\"header.jasper\"", ""),
        json!({"company": "North"}),
    );
    assert!(texts(&output).iter().any(|item| item == "North"));
    assert!(output
        .warnings
        .iter()
        .any(|warning| warning.tag.contains("header.jasper")));
}

#[test]
fn a_missing_file_warns_and_keeps_the_master() {
    let dir = TempDir::new();
    let output = render_in(&dir, &master("\"missing.jrxml\"", ""), json!({}));
    assert!(texts(&output).iter().any(|item| item == "Master"));
    assert!(output
        .warnings
        .iter()
        .any(|warning| warning.tag.contains("missing.jrxml")));
}

#[test]
fn a_cycle_does_not_recurse() {
    let dir = TempDir::new();
    dir.write(
        "loop.jrxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<jasperReport name="Loop" pageWidth="200" pageHeight="200" columnWidth="200" leftMargin="0" rightMargin="0" topMargin="0" bottomMargin="0">
  <detail>
    <band height="30">
      <staticText>
        <reportElement x="0" y="0" width="40" height="12"/>
        <text><![CDATA[Loop]]></text>
      </staticText>
      <subreport>
        <reportElement x="0" y="14" width="80" height="12"/>
        <subreportExpression><![CDATA["loop.jrxml"]]></subreportExpression>
      </subreport>
    </band>
  </detail>
</jasperReport>"#,
    );
    let output = render_in(&dir, &master("\"loop.jrxml\"", ""), json!({}));
    assert!(texts(&output).iter().any(|item| item == "Loop"));
    assert!(output
        .warnings
        .iter()
        .any(|warning| warning.tag == "subreport cycle"));
}

#[test]
fn the_element_round_trips() {
    let template = master("\"orders.jrxml\"", "customer.orders");
    let report = parse(&template).unwrap();
    let again = parse(&write_report(&report)).unwrap();
    let Element::Subreport(before) = &report.bands[0].elements[1] else {
        panic!("missing subreport");
    };
    let Element::Subreport(after) = &again.bands[0].elements[1] else {
        panic!("writer dropped the subreport");
    };
    assert_eq!(before.expression, "\"orders.jrxml\"");
    assert_eq!(before.data_source.as_deref(), Some("customer.orders"));
    assert_eq!(after, before);
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
}

#[test]
fn the_sample_fills_orders_from_the_template_directory() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../jrxmls");
    let template = fs::read_to_string(root.join("Subreport.jrxml")).unwrap();
    let document: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(root.join("SubreportData.json")).unwrap())
            .unwrap();
    let when = Utc.with_ymd_and_hms(2026, 10, 12, 0, 0, 0).unwrap();
    let output = render_with_assets(&template, &document, when, Some(&root)).unwrap();
    assert!(output.pdf.starts_with(b"%PDF"));
    assert!(
        output
            .warnings
            .iter()
            .all(|warning| !warning.tag.starts_with("subreport")),
        "{:?}",
        output.warnings
    );
    let text = pdf_extract::extract_text_from_mem(&output.pdf).unwrap();
    for label in [
        "Orders",
        "Northwind Traders",
        "Maria Anders",
        "SO-1001",
        "SO-1002",
        "BOLT-1",
        "NUT-2",
        "WASHER-3",
        "SKU",
        "Qty",
    ] {
        assert!(text.contains(label), "missing {label} in {text:?}");
    }
}
