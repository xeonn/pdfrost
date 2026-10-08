use chrono::{TimeZone, Utc};
use pdfrost_engine::{
    parse, render, write_report, BandKind, Element, LineSpacing, PlacementKind, PositionType,
};
use serde_json::json;

fn when() -> chrono::DateTime<chrono::Utc> {
    Utc.with_ymd_and_hms(2026, 10, 8, 9, 0, 0).unwrap()
}

fn report(body: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<jasperReport name="Layout" pageWidth="200" pageHeight="200" columnWidth="180" leftMargin="10" rightMargin="10" topMargin="10" bottomMargin="10">
{body}
</jasperReport>"#
    )
}

fn text(name: &str, x: f32, y: f32) -> String {
    format!(
        r#"<staticText>
        <reportElement x="{x}" y="{y}" width="40" height="12"/>
        <text><![CDATA[{name}]]></text>
      </staticText>"#
    )
}

#[test]
fn element_group_flattens_into_the_band() {
    let xml = report(&format!(
        r#"<title>
    <band height="40" splitType="Stretch">
      <elementGroup>
        {hello}
        <elementGroup>
          {nested}
        </elementGroup>
      </elementGroup>
    </band>
  </title>"#,
        hello = text("Hello", 0.0, 0.0),
        nested = text("Nested", 0.0, 14.0),
    ));
    let parsed = parse(&xml).unwrap();
    assert!(parsed
        .warnings
        .iter()
        .all(|warning| warning.tag != "elementGroup"));
    let title = parsed
        .bands
        .iter()
        .find(|band| band.kind == BandKind::Title)
        .unwrap();
    let labels: Vec<_> = title
        .elements
        .iter()
        .filter_map(|element| match element {
            Element::StaticText(text) => Some(text.text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(labels, ["Hello", "Nested"]);

    let output = render(&xml, &json!({}), when()).unwrap();
    let hello = output
        .placements
        .iter()
        .find(|placement| placement.text.as_deref() == Some("Hello"))
        .unwrap();
    assert_eq!(hello.page, 1);
    assert_eq!(hello.x, 10.0);
    assert_eq!(hello.y, 10.0);
    assert!(!write_report(&parsed).contains("elementGroup"));
}

#[test]
fn ellipse_uses_the_graphic_pen_and_stays_in_its_box() {
    let xml = report(
        r##"<title>
    <band height="50" splitType="Stretch">
      <ellipse>
        <reportElement x="4" y="6" width="30" height="20" mode="Opaque" backcolor="#FF0000"/>
        <graphicElement>
          <pen lineWidth="2.0" lineColor="#0000FF"/>
        </graphicElement>
      </ellipse>
    </band>
  </title>"##,
    );
    let parsed = parse(&xml).unwrap();
    assert!(parsed
        .warnings
        .iter()
        .all(|warning| warning.tag != "ellipse" && warning.tag != "graphicElement"));
    let Element::Ellipse(graphic) = &parsed.bands[0].elements[0] else {
        panic!("ellipse");
    };
    let pen = graphic
        .element
        .borders
        .as_ref()
        .and_then(|borders| borders.pen.as_ref());
    assert_eq!(pen.and_then(|pen| pen.line_width), Some(2.0));

    let output = render(&xml, &json!({}), when()).unwrap();
    assert!(output.pdf.starts_with(b"%PDF-"));
    let placed = output
        .placements
        .iter()
        .find(|placement| placement.kind == PlacementKind::Ellipse)
        .unwrap();
    assert_eq!(placed.page, 1);
    assert_eq!(placed.x, 14.0);
    assert_eq!(placed.y, 16.0);
    assert_eq!(placed.width, 30.0);
    assert_eq!(placed.height, 20.0);
    assert_eq!(placed.stroke_width, Some(2.0));

    let again = parse(&write_report(&parsed)).unwrap();
    assert!(matches!(again.bands[0].elements[0], Element::Ellipse(_)));
}

#[test]
fn column_footer_sits_above_the_page_footer_and_shrinks_the_page() {
    let xml = report(&format!(
        r#"<title>
    <band height="100" splitType="Stretch">{body}</band>
  </title>
  <summary>
    <band height="40" splitType="Stretch">{more}</band>
  </summary>
  <columnFooter>
    <band height="30" splitType="Stretch">{col}</band>
  </columnFooter>
  <pageFooter>
    <band height="20" splitType="Stretch">{page}</band>
  </pageFooter>"#,
        body = text("Body", 0.0, 0.0),
        more = text("More", 0.0, 0.0),
        col = text("Col", 0.0, 0.0),
        page = text("Page", 0.0, 0.0),
    ));
    let output = render(&xml, &json!({}), when()).unwrap();
    let at = |label: &str, page: u32| {
        output
            .placements
            .iter()
            .find(|placement| placement.page == page && placement.text.as_deref() == Some(label))
            .unwrap_or_else(|| panic!("{label} on page {page}"))
    };
    assert_eq!(at("Body", 1).y, 10.0);
    assert_eq!(at("Col", 1).y, 140.0);
    assert_eq!(at("Page", 1).y, 170.0);
    assert_eq!(at("More", 2).y, 10.0);
    assert_eq!(at("Col", 2).y, 140.0);
    assert_eq!(at("Page", 2).y, 170.0);

    let written = parse(&write_report(&parse(&xml).unwrap())).unwrap();
    assert!(written
        .bands
        .iter()
        .any(|band| band.kind == BandKind::ColumnFooter));
}

#[test]
fn a_break_continues_the_band_on_the_next_page() {
    for kind in ["Page", "Column"] {
        let xml = report(&format!(
            r#"<title>
    <band height="40" splitType="Stretch">
      {before}
      <break type="{kind}">
        <reportElement x="0" y="16" width="100" height="1"/>
      </break>
      {after}
    </band>
  </title>"#,
            before = text("Before", 0.0, 0.0),
            after = text("After", 0.0, 10.0),
        ));
        let output = render(&xml, &json!({}), when()).unwrap();
        let before = output
            .placements
            .iter()
            .find(|placement| placement.text.as_deref() == Some("Before"))
            .unwrap();
        let after = output
            .placements
            .iter()
            .find(|placement| placement.text.as_deref() == Some("After"))
            .unwrap();
        assert_eq!(before.page, 1, "{kind}");
        assert_eq!(before.y, 10.0, "{kind}");
        assert_eq!(after.page, 2, "{kind}");
        assert_eq!(after.y, 20.0, "{kind}");
    }

    let xml = report(
        r#"<title>
    <band height="40" splitType="Stretch">
      <break type="Column">
        <reportElement x="0" y="0" width="1" height="1"/>
      </break>
    </band>
  </title>"#,
    );
    let parsed = parse(&xml).unwrap();
    let Element::Break(page_break) = &parsed.bands[0].elements[0] else {
        panic!("break");
    };
    assert!(page_break.column);
    let written = write_report(&parsed);
    assert!(written.contains(r#"<break type="Column">"#));
    assert!(matches!(
        parse(&written).unwrap().bands[0].elements[0],
        Element::Break(_)
    ));
}

#[test]
fn stretched_text_grows_and_a_float_element_moves_with_it() {
    let xml = report(
        r#"<title>
    <band height="40" splitType="Stretch">
      <staticText isStretchWithOverflow="true">
        <reportElement x="0" y="0" width="80" height="10"/>
        <box padding="2" leftPadding="4"/>
        <textElement rotation="None">
          <font fontName="Helvetica" size="10" isItalic="true" isUnderline="true"/>
          <paragraph lineSpacing="Double"/>
        </textElement>
        <text><![CDATA[Hello
World]]></text>
      </staticText>
      <staticText>
        <reportElement positionType="Float" x="0" y="12" width="40" height="10"/>
        <text><![CDATA[Next]]></text>
      </staticText>
    </band>
  </title>"#,
    );
    let parsed = parse(&xml).unwrap();
    assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
    let Element::StaticText(first) = &parsed.bands[0].elements[0] else {
        panic!("text");
    };
    assert!(first.stretch && first.text_style.italic && first.text_style.underline);
    assert_eq!(first.text_style.line_spacing, Some(LineSpacing::Double));
    assert_eq!(first.text_style.font_name.as_deref(), Some("Helvetica"));
    let borders = first.element.borders.as_ref().unwrap();
    assert_eq!(borders.padding_left, 4.0);
    assert_eq!(borders.padding_top, 2.0);
    let Element::StaticText(next) = &parsed.bands[0].elements[1] else {
        panic!("next");
    };
    assert_eq!(next.element.position, PositionType::Float);

    let output = render(&xml, &json!({}), when()).unwrap();
    let hello = output
        .placements
        .iter()
        .find(|placement| placement.text.as_deref() == Some("Hello\nWorld"))
        .unwrap();
    let next_box = output
        .placements
        .iter()
        .find(|placement| placement.text.as_deref() == Some("Next"))
        .unwrap();
    assert!(hello.height > 10.0);
    let extra = hello.height - 10.0;
    assert!((next_box.y - (10.0 + 12.0 + extra)).abs() < 0.2);

    let written = write_report(&parsed);
    assert!(written.contains(r#"isStretchWithOverflow="true""#));
    assert!(written.contains(r#"positionType="Float""#));
    assert!(written.contains(r#"isItalic="true""#));
    assert!(written.contains(r#"isUnderline="true""#));
    assert!(written.contains(r#"lineSpacing="Double""#));
    assert!(written.contains(r#"leftPadding="4""#));
    let round_trip = parse(&written).unwrap();
    assert!(round_trip.warnings.is_empty(), "{:?}", round_trip.warnings);
}

#[test]
fn paragraph_markup_and_strike_round_trip() {
    let xml = report(
        r#"<title>
    <band height="40" splitType="Stretch">
      <staticText>
        <reportElement x="0" y="0" width="120" height="20"/>
        <textElement markup="styled">
          <font size="10" isStrikeThrough="true"/>
          <paragraph firstLineIndent="6" spacingBefore="4" tabStopWidth="30">
            <tabStop position="50" alignment="Right"/>
          </paragraph>
        </textElement>
        <text><![CDATA[A	B]]></text>
      </staticText>
    </band>
  </title>"#,
    );
    let parsed = parse(&xml).unwrap();
    assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
    let Element::StaticText(text) = &parsed.bands[0].elements[0] else {
        panic!("text");
    };
    assert_eq!(text.text_style.markup, pdfrost_engine::Markup::Styled);
    assert!(text.text_style.strike_through);
    assert_eq!(text.text_style.first_line_indent, Some(6.0));
    assert_eq!(text.text_style.spacing_before, Some(4.0));
    assert_eq!(text.text_style.tab_stop_width, Some(30.0));
    assert_eq!(text.text_style.tab_stops.len(), 1);
    assert_eq!(text.text_style.tab_stops[0].position, 50.0);
    assert_eq!(
        text.text_style.tab_stops[0].alignment,
        pdfrost_engine::TabStopAlign::Right
    );
    let written = write_report(&parsed);
    assert!(written.contains(r#"markup="styled""#));
    assert!(written.contains(r#"isStrikeThrough="true""#));
    assert!(written.contains(r#"firstLineIndent="6""#));
    assert!(written.contains(r#"spacingBefore="4""#));
    assert!(written.contains(r#"alignment="Right""#));
    let round_trip = parse(&written).unwrap();
    assert!(round_trip.warnings.is_empty(), "{:?}", round_trip.warnings);
}
