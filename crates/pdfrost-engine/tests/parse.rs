use std::fs;
use std::path::PathBuf;

use pdfrost_engine::{
    parse, BandKind, Color, Element, Error, Mode, SectionKind, SplitType, TextAlignment,
};

fn jrxmls_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../jrxmls")
}

fn read_template(name: &str) -> String {
    let path = jrxmls_dir().join(name);
    fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()))
}

#[test]
fn every_template_round_trips_through_json() {
    let mut names = Vec::new();
    for entry in fs::read_dir(jrxmls_dir()).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.ends_with(".jrxml") {
            names.push(name);
        }
    }
    assert!(names.len() >= 7, "{names:?}");
    for name in names {
        let report = parse(&read_template(&name)).unwrap_or_else(|err| panic!("{name}: {err}"));
        let json = serde_json::to_value(&report).unwrap();
        let back: pdfrost_engine::Report = serde_json::from_value(json).unwrap();
        assert_eq!(report, back, "{name}");
    }
}

#[test]
fn every_template_parses() {
    let mut names = Vec::new();
    for entry in fs::read_dir(jrxmls_dir()).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.ends_with(".jrxml") {
            names.push(name);
        }
    }
    names.sort();
    assert!(names.len() >= 7, "{names:?}");

    for name in names {
        let report = parse(&read_template(&name)).unwrap_or_else(|err| panic!("{name}: {err}"));
        for warning in &report.warnings {
            assert!(
                !matches!(
                    warning.tag.as_str(),
                    "queryString" | "group" | "groupExpression" | "property"
                ),
                "{name} warned about {}",
                warning.tag
            );
        }
    }
}

#[test]
fn sample_keeps_page_pen_text_and_chart() {
    let report = parse(&read_template("Sample.jrxml")).unwrap();
    assert_eq!(report.page_width, 595.0);
    assert_eq!(report.page_height, 842.0);
    assert_eq!(report.column_width, 555.0);
    assert_eq!(report.left_margin, 20.0);

    let style = report
        .styles
        .iter()
        .find(|style| style.name == "Table_TH")
        .unwrap();
    assert_eq!(style.mode, Some(Mode::Opaque));
    assert_eq!(
        style.backcolor,
        Some(Color {
            r: 0xF0,
            g: 0xF8,
            b: 0xFF
        })
    );
    let pen = style.borders.as_ref().unwrap().pen.as_ref().unwrap();
    assert_eq!(pen.line_width, Some(0.5));
    assert_eq!(pen.line_color, Some(Color { r: 0, g: 0, b: 0 }));
    assert_eq!(pen.line_style, None);

    let title = report
        .bands
        .iter()
        .find(|band| band.kind == BandKind::Title)
        .unwrap();
    assert_eq!(title.height, 70.0);
    assert_eq!(title.split_type, SplitType::Stretch);
    let Element::StaticText(text) = &title.elements[0] else {
        panic!("title starts with static text");
    };
    assert_eq!(text.text, "Go binding to Jasper Report");
    assert_eq!(text.text_style.text_alignment, Some(TextAlignment::Center));
    assert_eq!(text.text_style.font_size, Some(14.0));
    assert_eq!(text.element.x, 0.0);
    assert_eq!(text.element.width, 555.0);

    let summary = report
        .bands
        .iter()
        .find(|band| band.kind == BandKind::Summary)
        .unwrap();
    assert!(matches!(
        summary.elements.as_slice(),
        [
            Element::Rectangle(_),
            Element::Line(_),
            Element::Chart(_),
            Element::Chart(_),
            Element::Chart(_),
            Element::Chart(_),
            Element::Chart(_),
            Element::Table(_)
        ]
    ));
    let kinds: Vec<_> = summary
        .elements
        .iter()
        .filter_map(|element| match element {
            Element::Chart(chart) => Some(chart.kind.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        kinds,
        [
            "areaChart",
            "barChart",
            "lineChart",
            "stackedAreaChart",
            "stackedBarChart"
        ]
    );
    let Element::Chart(chart_box) = &summary.elements[2] else {
        panic!("summary keeps the chart box");
    };
    assert_eq!(chart_box.element.x, 0.0);
    assert_eq!(chart_box.element.y, 0.0);
    assert_eq!(chart_box.element.width, 270.0);
    assert_eq!(chart_box.element.height, 148.0);
    assert_eq!(chart_box.evaluation_time.as_deref(), Some("Report"));
    assert!(chart_box.show_legend);
    assert_eq!(chart_box.title.as_deref(), Some("\"Area\""));
    assert_eq!(chart_box.data_source.as_deref(), Some("chartPoints"));
    assert_eq!(chart_box.sub_dataset.as_deref(), Some("ChartPoints"));
    assert_eq!(chart_box.series.len(), 2);
    assert_eq!(chart_box.series[0].series_expression, "\"Units\"");
    assert_eq!(chart_box.series[0].category_expression, "$F{month}");
    assert_eq!(chart_box.series[0].value_expression, "$F{units}");
    assert_eq!(chart_box.series[1].series_expression, "\"Returns\"");
    assert_eq!(chart_box.series[1].value_expression, "$F{returns}");
    assert!(report.warnings.iter().all(|warning| warning.tag != "chart"));
}

#[test]
fn shipment_keeps_table_fields_on_the_subdataset() {
    let report = parse(&read_template("ShipmentOut.jrxml")).unwrap();
    assert!(report.fields.iter().all(|field| field.name != "productid"));
    assert!(report.fields.iter().any(|field| field.name == "docno"));

    let dataset = report
        .subdatasets
        .iter()
        .find(|dataset| dataset.name == "ShipmentOutDataset")
        .unwrap();
    let product_id = dataset
        .fields
        .iter()
        .find(|field| field.name == "productid")
        .unwrap();
    assert_eq!(product_id.json_path.as_deref(), Some("product.id"));
    assert!(!product_id.is_integer());
    let qty = dataset
        .fields
        .iter()
        .find(|field| field.name == "qty")
        .unwrap();
    assert!(qty.is_integer());

    let detail = report
        .bands
        .iter()
        .find(|band| band.kind == BandKind::Detail)
        .unwrap();
    let Element::Table(table) = &detail.elements[0] else {
        panic!("detail starts with the table");
    };
    assert_eq!(table.sub_dataset.as_deref(), Some("ShipmentOutDataset"));
    assert_eq!(table.data_source.as_deref(), Some("itemsList"));
    assert_eq!(table.element.x, 0.0);
    assert_eq!(table.element.y, 20.0);
    assert_eq!(table.columns[0].width, 138.0);

    let header = table.columns[0]
        .sections
        .iter()
        .find(|section| section.kind == SectionKind::ColumnHeader)
        .unwrap();
    assert_eq!(header.style.as_deref(), Some("Table_CH"));
    assert_eq!(header.height, 30.0);
    let Element::StaticText(label) = &header.elements[0] else {
        panic!("column header label");
    };
    assert_eq!(label.text, "Item No");
    assert!(label.text_style.bold);
}

#[test]
fn receive_keeps_an_empty_column_footer() {
    let report = parse(&read_template("Receive.jrxml")).unwrap();
    let table = report
        .bands
        .iter()
        .find_map(|band| {
            band.elements.iter().find_map(|element| match element {
                Element::Table(table) => Some(table),
                _ => None,
            })
        })
        .unwrap();
    assert_eq!(table.when_no_data.as_deref(), Some("AllSectionsNoDetail"));
    let footer = table.columns[0]
        .sections
        .iter()
        .find(|section| section.kind == SectionKind::ColumnFooter)
        .unwrap();
    assert_eq!(footer.height, 30.0);
    assert_eq!(footer.style.as_deref(), Some("Table 1_CH"));
    assert!(footer.elements.is_empty());
}

#[test]
fn broken_xml_and_wrong_root_fail() {
    assert!(matches!(parse("<jasperReport"), Err(Error::Xml(_))));
    assert_eq!(parse("<report/>").unwrap_err(), Error::NotAReport);
}
