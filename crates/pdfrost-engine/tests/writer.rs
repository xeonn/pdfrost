use pdfrost_engine::{
    write_report, Band, BandKind, Color, Column, ColumnSection, Element, Field, Graphic, Mode, Pen,
    Report, ReportElement, SectionKind, SplitType, StaticText, Style, SubDataset, Table, TextField,
    TextStyle,
};

#[test]
fn a_small_report_round_trips_through_the_writer() {
    let report = Report {
        name: "RoundTrip".to_string(),
        page_width: 200.0,
        page_height: 300.0,
        column_width: 180.0,
        left_margin: 10.0,
        right_margin: 10.0,
        top_margin: 12.0,
        bottom_margin: 12.0,
        styles: vec![Style {
            name: "Table_CH".to_string(),
            mode: Some(Mode::Opaque),
            backcolor: Some(Color {
                r: 191,
                g: 225,
                b: 255,
            }),
            borders: Some(pdfrost_engine::Box {
                pen: Some(Pen {
                    line_width: Some(0.5),
                    line_color: Some(Color { r: 0, g: 0, b: 0 }),
                    line_style: Some("Solid".to_string()),
                }),
                ..pdfrost_engine::Box::default()
            }),
        }],
        fields: vec![Field {
            name: "docno".to_string(),
            class_name: "java.lang.String".to_string(),
            description: None,
            json_path: Some("docno".to_string()),
        }],
        query: None,
        subdatasets: vec![SubDataset {
            name: "Items".to_string(),
            fields: vec![Field {
                name: "sku".to_string(),
                class_name: "java.lang.String".to_string(),
                description: None,
                json_path: Some("sku".to_string()),
            }],
        }],
        bands: vec![
            Band {
                kind: BandKind::Title,
                height: 40.0,
                split_type: SplitType::Stretch,
                elements: vec![
                    Element::StaticText(StaticText {
                        element: ReportElement {
                            x: 0.0,
                            y: 0.0,
                            width: 100.0,
                            height: 16.0,
                            ..ReportElement::default()
                        },
                        text_style: TextStyle::default(),
                        text: "Hello & <world>".to_string(),
                        stretch: false,
                    }),
                    Element::TextField(TextField {
                        element: ReportElement {
                            x: 0.0,
                            y: 18.0,
                            width: 80.0,
                            height: 14.0,
                            ..ReportElement::default()
                        },
                        text_style: TextStyle::default(),
                        expression: "$F{docno}".to_string(),
                        stretch: false,
                    }),
                    Element::Line(Graphic {
                        element: ReportElement {
                            x: 0.0,
                            y: 34.0,
                            width: 80.0,
                            height: 1.0,
                            ..ReportElement::default()
                        },
                    }),
                    Element::Rectangle(Graphic {
                        element: ReportElement {
                            x: 90.0,
                            y: 0.0,
                            width: 20.0,
                            height: 12.0,
                            ..ReportElement::default()
                        },
                    }),
                ],
            },
            Band {
                kind: BandKind::Detail,
                height: 60.0,
                split_type: SplitType::Stretch,
                elements: vec![Element::Table(Table {
                    element: ReportElement {
                        x: 0.0,
                        y: 0.0,
                        width: 80.0,
                        height: 60.0,
                        ..ReportElement::default()
                    },
                    when_no_data: Some("AllSectionsNoDetail".to_string()),
                    sub_dataset: Some("Items".to_string()),
                    data_source: Some("itemsList".to_string()),
                    columns: vec![Column {
                        width: 80.0,
                        sections: vec![
                            ColumnSection {
                                kind: SectionKind::ColumnHeader,
                                style: Some("Table_CH".to_string()),
                                height: 30.0,
                                elements: vec![Element::StaticText(StaticText {
                                    element: ReportElement {
                                        width: 80.0,
                                        height: 30.0,
                                        ..ReportElement::default()
                                    },
                                    text_style: TextStyle::default(),
                                    text: "Item".to_string(),
                                    stretch: false,
                                })],
                            },
                            ColumnSection {
                                kind: SectionKind::DetailCell,
                                style: None,
                                height: 30.0,
                                elements: vec![Element::TextField(TextField {
                                    element: ReportElement {
                                        width: 80.0,
                                        height: 30.0,
                                        ..ReportElement::default()
                                    },
                                    text_style: TextStyle::default(),
                                    expression: "$F{sku}".to_string(),
                                    stretch: false,
                                })],
                            },
                            ColumnSection {
                                kind: SectionKind::ColumnFooter,
                                style: None,
                                height: 30.0,
                                elements: Vec::new(),
                            },
                        ],
                    }],
                })],
            },
        ],
        warnings: Vec::new(),
    };

    let xml = write_report(&report);
    let parsed = pdfrost_engine::parse(&xml).unwrap();
    assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
    assert_eq!(parsed.name, report.name);
    assert_eq!(parsed.page_width, report.page_width);
    assert_eq!(parsed.page_height, report.page_height);
    assert_eq!(parsed.left_margin, report.left_margin);
    assert_eq!(parsed.styles, report.styles);
    assert_eq!(parsed.fields, report.fields);
    assert_eq!(parsed.subdatasets, report.subdatasets);
    assert_eq!(parsed.bands.len(), report.bands.len());
    assert_eq!(parsed.bands[0].elements[0], report.bands[0].elements[0]);
    assert_eq!(parsed.bands[0].elements[1], report.bands[0].elements[1]);
    assert_eq!(parsed.bands[0].elements[2], report.bands[0].elements[2]);
    assert_eq!(parsed.bands[0].elements[3], report.bands[0].elements[3]);
    let Element::Table(table) = &parsed.bands[1].elements[0] else {
        panic!("detail is not a table");
    };
    assert_eq!(table.when_no_data.as_deref(), Some("AllSectionsNoDetail"));
    assert_eq!(table.sub_dataset.as_deref(), Some("Items"));
    assert_eq!(table.data_source.as_deref(), Some("itemsList"));
    assert_eq!(table.columns[0].width, 80.0);
    assert_eq!(
        table.columns[0].sections[0].style.as_deref(),
        Some("Table_CH")
    );
    assert_eq!(table.columns[0].sections[0].height, 30.0);
    assert_eq!(table.columns[0].sections[2].height, 30.0);
    assert!(table.columns[0].sections[2].elements.is_empty());
}

#[test]
fn a_root_field_path_is_written_relative_to_the_query() {
    let mut report = pdfrost_engine::Report {
        name: "Customers".to_string(),
        page_width: 200.0,
        page_height: 200.0,
        column_width: 160.0,
        left_margin: 20.0,
        right_margin: 20.0,
        top_margin: 20.0,
        bottom_margin: 20.0,
        styles: Vec::new(),
        fields: vec![Field {
            name: "Address".to_string(),
            class_name: "java.lang.String".to_string(),
            description: Some("Address".to_string()),
            json_path: Some("(root).Northwind.Customers.Address".to_string()),
        }],
        query: Some("Northwind.Customers".to_string()),
        subdatasets: Vec::new(),
        bands: Vec::new(),
        warnings: Vec::new(),
    };
    let xml = write_report(&report);
    assert!(xml.contains("value=\"Address\""));
    assert!(!xml.contains("(root)"));
    report.fields[0].json_path = Some("Address".to_string());
    let parsed = pdfrost_engine::parse(&xml).unwrap();
    assert_eq!(parsed.fields, report.fields);
    assert_eq!(parsed.query.as_deref(), Some("Northwind.Customers"));
}
