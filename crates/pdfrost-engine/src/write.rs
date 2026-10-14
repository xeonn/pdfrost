//! Write the supported JRXML subset back to XML.
//!
//! The parser accepts this output. Studio metadata and unsupported tags are not written.
//! A supported chart is written back with its category series. Other chart tags keep the box only.

use std::fmt::Write as _;

use crate::model::*;

pub fn write_report(report: &Report) -> String {
    let mut xml = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    xml.push('\n');
    let _ = writeln!(
        xml,
        "<jasperReport xmlns=\"http://jasperreports.sourceforge.net/jasperreports\" xmlns:jr=\"http://jasperreports.sourceforge.net/jasperreports/components\" name=\"{}\" pageWidth=\"{}\" pageHeight=\"{}\" columnWidth=\"{}\" leftMargin=\"{}\" rightMargin=\"{}\" topMargin=\"{}\" bottomMargin=\"{}\">",
        escape(&report.name),
        num(report.page_width),
        num(report.page_height),
        num(report.column_width),
        num(report.left_margin),
        num(report.right_margin),
        num(report.top_margin),
        num(report.bottom_margin),
    );
    for style in &report.styles {
        write_style(&mut xml, style);
    }
    for dataset in &report.subdatasets {
        write_subdataset(&mut xml, report, dataset);
    }
    if let Some(query) = &report.query {
        let _ = writeln!(
            xml,
            "\t<queryString language=\"JSON\">{}</queryString>",
            escape(query)
        );
    }
    for field in &report.fields {
        write_field(&mut xml, field, 1, report.query.as_deref());
    }
    for band in &report.bands {
        write_band(&mut xml, band);
    }
    xml.push_str("</jasperReport>\n");
    xml
}

fn write_style(xml: &mut String, style: &Style) {
    let _ = writeln!(xml, "\t<style name=\"{}\"", escape(&style.name));
    if let Some(mode) = style.mode {
        let _ = write!(xml, " mode=\"{}\"", mode_name(mode));
    }
    if let Some(color) = style.backcolor {
        let _ = write!(xml, " backcolor=\"{}\"", hex(color));
    }
    if style.borders.is_none() {
        xml.push_str("/>");
        return;
    }
    xml.push_str(">\n");
    if let Some(borders) = &style.borders {
        write_box(xml, borders, 2);
    }
    xml.push_str("\t</style>\n");
}

fn write_subdataset(xml: &mut String, report: &Report, dataset: &SubDataset) {
    let _ = writeln!(xml, "\t<subDataset name=\"{}\">", escape(&dataset.name));
    let source = dataset_source(report, &dataset.name);
    for field in &dataset.fields {
        write_field(xml, field, 2, source);
    }
    xml.push_str("\t</subDataset>\n");
}

fn write_field(xml: &mut String, field: &Field, indent: usize, record_path: Option<&str>) {
    let pad = "\t".repeat(indent);
    let _ = writeln!(
        xml,
        "{pad}<field name=\"{}\" class=\"{}\">",
        escape(&field.name),
        escape(&field.class_name)
    );
    if let Some(path) = field.json_path.as_deref() {
        let expression = relative_expression(path, record_path);
        let _ = writeln!(
            xml,
            "{pad}\t<property name=\"net.sf.jasperreports.json.field.expression\" value=\"{}\"/>",
            escape(&expression)
        );
    }
    if let Some(description) = &field.description {
        let _ = writeln!(
            xml,
            "{pad}\t<fieldDescription>{}</fieldDescription>",
            escape(description)
        );
    }
    let _ = writeln!(xml, "{pad}</field>");
}

/// Studio's JSON field expression is relative to the query. `(root).Northwind.Customers.Address` is `Address`.
fn relative_expression(path: &str, record_path: Option<&str>) -> String {
    let mut rest = path.trim();
    if let Some(after) = rest.strip_prefix("(root)") {
        rest = after.trim_start_matches('.');
    }
    let Some(record) = record_path
        .map(str::trim)
        .filter(|record| !record.is_empty() && *record != ".")
    else {
        return rest.to_string();
    };
    let compact = record
        .split('.')
        .map(str::trim)
        .filter(|part| !part.is_empty() && !part.bytes().all(|byte| byte.is_ascii_digit()))
        .collect::<Vec<_>>()
        .join(".");
    let prefix = format!("{compact}.");
    if !compact.is_empty() && rest.starts_with(&prefix) {
        return rest[prefix.len()..].to_string();
    }
    rest.to_string()
}

fn dataset_source<'a>(report: &'a Report, name: &str) -> Option<&'a str> {
    for band in &report.bands {
        for element in &band.elements {
            let (dataset, source) = match element {
                Element::Table(table) => {
                    (table.sub_dataset.as_deref(), table.data_source.as_deref())
                }
                Element::Chart(chart) => {
                    (chart.sub_dataset.as_deref(), chart.data_source.as_deref())
                }
                _ => continue,
            };
            if dataset == Some(name) {
                if let Some(source) = source.map(str::trim).filter(|source| !source.is_empty()) {
                    return Some(source);
                }
            }
        }
    }
    None
}

fn write_band(xml: &mut String, band: &Band) {
    let _ = writeln!(
        xml,
        "\t<{}>\n\t\t<band height=\"{}\" splitType=\"Stretch\">",
        band.kind.name(),
        num(band.height)
    );
    for element in &band.elements {
        write_element(xml, element, 3);
    }
    xml.push_str("\t\t</band>\n\t</");
    xml.push_str(band.kind.name());
    xml.push_str(">\n");
}

fn write_element(xml: &mut String, element: &Element, indent: usize) {
    match element {
        Element::StaticText(text) => {
            write_text_element(
                xml,
                indent,
                "staticText",
                &text.element,
                &text.text_style,
                &text.text,
                false,
                text.stretch,
            );
        }
        Element::TextField(field) => {
            write_text_element(
                xml,
                indent,
                "textField",
                &field.element,
                &field.text_style,
                &field.expression,
                true,
                field.stretch,
            );
        }
        Element::Line(graphic) => write_graphic(xml, indent, "line", &graphic.element),
        Element::Rectangle(graphic) => write_graphic(xml, indent, "rectangle", &graphic.element),
        Element::Ellipse(graphic) => write_graphic(xml, indent, "ellipse", &graphic.element),
        Element::Image(image) => write_image(xml, indent, image),
        Element::Break(page_break) => write_break(xml, indent, page_break),
        Element::Chart(chart) => write_chart(xml, indent, chart),
        Element::Subreport(subreport) => write_subreport(xml, indent, subreport),
        Element::Table(table) => write_table(xml, indent, table),
    }
}

fn write_text_element(
    xml: &mut String,
    indent: usize,
    tag: &str,
    element: &ReportElement,
    style: &TextStyle,
    text: &str,
    expression: bool,
    stretch: bool,
) {
    let pad = "\t".repeat(indent);
    let _ = write!(xml, "{pad}<{tag}");
    if stretch {
        xml.push_str(" isStretchWithOverflow=\"true\"");
    }
    xml.push_str(">\n");
    write_report_element(xml, element, indent + 1);
    if let Some(borders) = &element.borders {
        write_box(xml, borders, indent + 1);
    }
    write_text_style(xml, style, indent + 1);
    let body = if expression {
        "textFieldExpression"
    } else {
        "text"
    };
    let _ = writeln!(xml, "{pad}\t<{body}>{}</{body}>", escape(text));
    let _ = writeln!(xml, "{pad}</{tag}>");
}

fn write_chart(xml: &mut String, indent: usize, chart: &Chart) {
    let pad = "\t".repeat(indent);
    let tag = chart_tag(&chart.kind);
    let _ = writeln!(xml, "{pad}<{tag}>");
    let _ = write!(xml, "{pad}\t<chart");
    if let Some(time) = &chart.evaluation_time {
        let _ = write!(xml, " evaluationTime=\"{}\"", escape(time));
    }
    if !chart.show_legend {
        xml.push_str(" isShowLegend=\"false\"");
    }
    xml.push_str(">\n");
    write_report_element(xml, &chart.element, indent + 2);
    if let Some(borders) = &chart.element.borders {
        write_box(xml, borders, indent + 2);
    }
    write_chart_text(
        xml,
        indent + 2,
        "chartTitle",
        "titleExpression",
        chart.title.as_deref(),
    );
    write_chart_text(
        xml,
        indent + 2,
        "chartSubtitle",
        "subtitleExpression",
        chart.subtitle.as_deref(),
    );
    if chart.show_legend {
        let _ = writeln!(xml, "{pad}\t\t<chartLegend/>");
    }
    let _ = writeln!(xml, "{pad}\t</chart>");
    if chart.data_source.is_some() || chart.sub_dataset.is_some() || !chart.series.is_empty() {
        let _ = writeln!(xml, "{pad}\t<categoryDataset>");
        if chart.data_source.is_some() || chart.sub_dataset.is_some() {
            let _ = writeln!(xml, "{pad}\t\t<dataset>");
            let _ = write!(xml, "{pad}\t\t\t<datasetRun");
            if let Some(name) = &chart.sub_dataset {
                let _ = write!(xml, " subDataset=\"{}\"", escape(name));
            }
            xml.push_str(">\n");
            if let Some(source) = &chart.data_source {
                let _ = writeln!(
                    xml,
                    "{pad}\t\t\t\t<dataSourceExpression>subDataSource(\"{}\")</dataSourceExpression>",
                    escape(source)
                );
            }
            let _ = writeln!(xml, "{pad}\t\t\t</datasetRun>");
            let _ = writeln!(xml, "{pad}\t\t</dataset>");
        }
        for series in &chart.series {
            let _ = writeln!(xml, "{pad}\t\t<categorySeries>");
            write_expression(
                xml,
                indent + 3,
                "seriesExpression",
                &series.series_expression,
            );
            write_expression(
                xml,
                indent + 3,
                "categoryExpression",
                &series.category_expression,
            );
            write_expression(xml, indent + 3, "valueExpression", &series.value_expression);
            let _ = writeln!(xml, "{pad}\t\t</categorySeries>");
        }
        let _ = writeln!(xml, "{pad}\t</categoryDataset>");
    }
    if let Some(plot) = plot_tag(&chart.kind) {
        let _ = writeln!(xml, "{pad}\t<{plot}>\n{pad}\t\t<plot/>\n{pad}\t</{plot}>");
    }
    let _ = writeln!(xml, "{pad}</{tag}>");
}

fn write_chart_text(
    xml: &mut String,
    indent: usize,
    tag: &str,
    expression_tag: &str,
    text: Option<&str>,
) {
    let Some(text) = text else {
        return;
    };
    let pad = "\t".repeat(indent);
    let _ = writeln!(xml, "{pad}<{tag}>");
    write_expression(xml, indent + 1, expression_tag, text);
    let _ = writeln!(xml, "{pad}</{tag}>");
}

fn write_expression(xml: &mut String, indent: usize, tag: &str, text: &str) {
    let pad = "\t".repeat(indent);
    let _ = writeln!(xml, "{pad}<{tag}>{}</{tag}>", escape(text));
}

fn plot_tag(kind: &str) -> Option<&'static str> {
    match kind {
        "areaChart" | "stackedAreaChart" => Some("areaPlot"),
        "barChart" | "stackedBarChart" => Some("barPlot"),
        "lineChart" => Some("linePlot"),
        _ => None,
    }
}

fn chart_tag(kind: &str) -> &str {
    if kind.len() > "Chart".len()
        && kind.ends_with("Chart")
        && kind.bytes().all(|byte| byte.is_ascii_alphanumeric())
    {
        kind
    } else {
        "areaChart"
    }
}

fn write_break(xml: &mut String, indent: usize, page_break: &PageBreak) {
    let pad = "\t".repeat(indent);
    let kind = if page_break.column { "Column" } else { "Page" };
    let _ = writeln!(xml, "{pad}<break type=\"{kind}\">");
    write_report_element(xml, &page_break.element, indent + 1);
    let _ = writeln!(xml, "{pad}</break>");
}

fn write_subreport(xml: &mut String, indent: usize, subreport: &Subreport) {
    let pad = "\t".repeat(indent);
    let _ = writeln!(xml, "{pad}<subreport>");
    write_report_element(xml, &subreport.element, indent + 1);
    if let Some(borders) = &subreport.element.borders {
        write_box(xml, borders, indent + 1);
    }
    if let Some(source) = &subreport.data_source {
        let _ = writeln!(
            xml,
            "{pad}\t<dataSourceExpression>subDataSource(\"{}\")</dataSourceExpression>",
            escape(source)
        );
    }
    if !subreport.expression.is_empty() {
        let _ = writeln!(
            xml,
            "{pad}\t<subreportExpression>{}</subreportExpression>",
            escape(&subreport.expression)
        );
    }
    let _ = writeln!(xml, "{pad}</subreport>");
}

fn write_image(xml: &mut String, indent: usize, image: &Image) {
    let pad = "\t".repeat(indent);
    let _ = write!(xml, "{pad}<image");
    if let Some(scale) = image.scale_image {
        let _ = write!(xml, " scaleImage=\"{}\"", scale_image_name(scale));
    }
    if let Some(rotation) = image.rotation {
        let _ = write!(xml, " rotation=\"{}\"", rotation_name(rotation));
    }
    if let Some(align) = image.h_align {
        let _ = write!(xml, " hAlign=\"{}\"", image_h_align_name(align));
    }
    if let Some(align) = image.v_align {
        let _ = write!(xml, " vAlign=\"{}\"", v_align(align));
    }
    if let Some(cache) = image.using_cache {
        let _ = write!(xml, " isUsingCache=\"{}\"", cache);
    }
    if image.lazy {
        xml.push_str(" isLazy=\"true\"");
    }
    if let Some(on_error) = image.on_error_type {
        let _ = write!(xml, " onErrorType=\"{}\"", on_error_name(on_error));
    }
    if let Some(time) = &image.evaluation_time {
        let _ = write!(xml, " evaluationTime=\"{}\"", escape(time));
    }
    if let Some(group) = &image.evaluation_group {
        let _ = write!(xml, " evaluationGroup=\"{}\"", escape(group));
    }
    xml.push_str(">\n");
    write_report_element_with(xml, &image.element, indent + 1, &image.pictures);
    if let Some(borders) = &image.element.borders {
        write_box(xml, borders, indent + 1);
    }
    if !image.expression.is_empty() {
        let _ = write!(xml, "{pad}\t<imageExpression");
        if let Some(class) = &image.expression_class {
            let _ = write!(xml, " class=\"{}\"", escape(class));
        }
        let _ = writeln!(xml, ">{}</imageExpression>", escape(&image.expression));
    }
    let _ = writeln!(xml, "{pad}</image>");
}

fn scale_image_name(scale: ScaleImage) -> &'static str {
    match scale {
        ScaleImage::Clip => "Clip",
        ScaleImage::FillFrame => "FillFrame",
        ScaleImage::RetainShape => "RetainShape",
        ScaleImage::RealHeight => "RealHeight",
        ScaleImage::RealSize => "RealSize",
    }
}

fn image_h_align_name(align: HorizontalImageAlign) -> &'static str {
    match align {
        HorizontalImageAlign::Left => "Left",
        HorizontalImageAlign::Center => "Center",
        HorizontalImageAlign::Right => "Right",
    }
}

fn on_error_name(on_error: OnErrorType) -> &'static str {
    match on_error {
        OnErrorType::Error => "Error",
        OnErrorType::Blank => "Blank",
        OnErrorType::Icon => "Icon",
    }
}

fn write_graphic(xml: &mut String, indent: usize, tag: &str, element: &ReportElement) {
    let pad = "\t".repeat(indent);
    let _ = writeln!(xml, "{pad}<{tag}>");
    write_report_element(xml, element, indent + 1);
    if let Some(borders) = &element.borders {
        write_box(xml, borders, indent + 1);
    }
    let _ = writeln!(xml, "{pad}</{tag}>");
}

fn write_table(xml: &mut String, indent: usize, table: &Table) {
    let pad = "\t".repeat(indent);
    xml.push_str(&pad);
    xml.push_str("<componentElement>\n");
    write_report_element(xml, &table.element, indent + 1);
    xml.push_str(&pad);
    xml.push_str("\t<jr:table");
    if let Some(when) = &table.when_no_data {
        let _ = writeln!(xml, " whenNoDataType=\"{}\"", escape(when));
    }
    xml.push('>');
    if table.sub_dataset.is_some() || table.data_source.is_some() {
        xml.push_str(&pad);
        xml.push_str("\t\t<datasetRun");
        if let Some(name) = &table.sub_dataset {
            let _ = writeln!(xml, " subDataset=\"{}\"", escape(name));
        }
        xml.push('>');
        if let Some(source) = &table.data_source {
            let _ = writeln!(
                xml,
                "{pad}\t\t\t<dataSourceExpression>subDataSource(\"{}\")</dataSourceExpression>",
                escape(source)
            );
        }
        xml.push_str(&pad);
        xml.push_str("\t\t</datasetRun>\n");
    }
    for column in &table.columns {
        let _ = writeln!(xml, "{pad}\t\t<jr:column width=\"{}\">", num(column.width));
        for section in &column.sections {
            write_section(xml, indent + 3, section);
        }
        let _ = writeln!(xml, "{pad}\t\t</jr:column>");
    }
    xml.push_str(&pad);
    xml.push_str("\t</jr:table>\n");
    xml.push_str(&pad);
    xml.push_str("</componentElement>\n");
}

fn write_section(xml: &mut String, indent: usize, section: &ColumnSection) {
    let pad = "\t".repeat(indent);
    let tag = match section.kind {
        SectionKind::TableHeader => "jr:tableHeader",
        SectionKind::ColumnHeader => "jr:columnHeader",
        SectionKind::DetailCell => "jr:detailCell",
        SectionKind::ColumnFooter => "jr:columnFooter",
        SectionKind::TableFooter => "jr:tableFooter",
    };
    let _ = writeln!(xml, "{pad}<{tag} height=\"{}\"", num(section.height));
    if let Some(style) = &section.style {
        let _ = write!(xml, " style=\"{}\"", escape(style));
    }
    if section.elements.is_empty() {
        xml.push_str("/>");
        return;
    }
    xml.push_str(">\n");
    for element in &section.elements {
        write_element(xml, element, indent + 1);
    }
    let _ = writeln!(xml, "{pad}</{tag}>");
}

fn write_report_element(xml: &mut String, element: &ReportElement, indent: usize) {
    write_report_element_with(xml, element, indent, &std::collections::BTreeMap::new());
}

fn write_report_element_with(
    xml: &mut String,
    element: &ReportElement,
    indent: usize,
    pictures: &std::collections::BTreeMap<String, String>,
) {
    let pad = "\t".repeat(indent);
    let _ = writeln!(
        xml,
        "{pad}<reportElement x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"",
        num(element.x),
        num(element.y),
        num(element.width),
        num(element.height)
    );
    if let Some(mode) = element.mode {
        let _ = write!(xml, " mode=\"{}\"", mode_name(mode));
    }
    if let Some(color) = element.backcolor {
        let _ = write!(xml, " backcolor=\"{}\"", hex(color));
    }
    if element.position != PositionType::FixRelativeToTop {
        let _ = write!(xml, " positionType=\"{}\"", position_name(element.position));
    }
    if pictures.is_empty() {
        xml.push_str("/>");
        return;
    }
    xml.push_str(">\n");
    for (name, value) in pictures {
        let _ = writeln!(
            xml,
            "{pad}\t<property name=\"{}\" value=\"{}\"/>",
            escape(name),
            escape(value)
        );
    }
    let _ = write!(xml, "{pad}</reportElement>");
}

fn write_box(xml: &mut String, borders: &Box, indent: usize) {
    let pad = "\t".repeat(indent);
    xml.push_str(&pad);
    xml.push_str("<box");
    let uniform = borders.padding_top == borders.padding_left
        && borders.padding_top == borders.padding_bottom
        && borders.padding_top == borders.padding_right;
    if uniform && borders.padding_top != 0.0 {
        let _ = write!(xml, " padding=\"{}\"", num(borders.padding_top));
    } else if !uniform {
        if borders.padding_top != 0.0 {
            let _ = write!(xml, " topPadding=\"{}\"", num(borders.padding_top));
        }
        if borders.padding_left != 0.0 {
            let _ = write!(xml, " leftPadding=\"{}\"", num(borders.padding_left));
        }
        if borders.padding_bottom != 0.0 {
            let _ = write!(xml, " bottomPadding=\"{}\"", num(borders.padding_bottom));
        }
        if borders.padding_right != 0.0 {
            let _ = write!(xml, " rightPadding=\"{}\"", num(borders.padding_right));
        }
    }
    xml.push_str(">\n");
    write_pen(xml, "pen", borders.pen.as_ref(), indent + 1);
    write_pen(xml, "topPen", borders.top.as_ref(), indent + 1);
    write_pen(xml, "leftPen", borders.left.as_ref(), indent + 1);
    write_pen(xml, "bottomPen", borders.bottom.as_ref(), indent + 1);
    write_pen(xml, "rightPen", borders.right.as_ref(), indent + 1);
    xml.push_str(&pad);
    xml.push_str("</box>\n");
}

fn write_pen(xml: &mut String, tag: &str, pen: Option<&Pen>, indent: usize) {
    let Some(pen) = pen else {
        return;
    };
    let pad = "\t".repeat(indent);
    let _ = writeln!(xml, "{pad}<{tag}");
    if let Some(width) = pen.line_width {
        let _ = write!(xml, " lineWidth=\"{}\"", num(width));
    }
    if let Some(color) = pen.line_color {
        let _ = write!(xml, " lineColor=\"{}\"", hex(color));
    }
    if let Some(style) = &pen.line_style {
        let _ = write!(xml, " lineStyle=\"{}\"", escape(style));
    }
    xml.push_str("/>");
}

fn write_text_style(xml: &mut String, style: &TextStyle, indent: usize) {
    if style.text_alignment.is_none()
        && style.vertical_alignment.is_none()
        && style.rotation.is_none()
        && style.font_size.is_none()
        && !style.bold
        && !style.italic
        && !style.underline
        && !style.strike_through
        && style.markup == Markup::None
        && style.font_name.is_none()
        && style.line_spacing.is_none()
        && style.line_spacing_size.is_none()
        && style.left_indent.is_none()
        && style.right_indent.is_none()
        && style.first_line_indent.is_none()
        && style.spacing_before.is_none()
        && style.tab_stop_width.is_none()
        && style.tab_stops.is_empty()
    {
        return;
    }
    let pad = "\t".repeat(indent);
    xml.push_str(&pad);
    xml.push_str("<textElement");
    if style.markup != Markup::None {
        let _ = write!(xml, " markup=\"{}\"", markup_name(style.markup));
    }
    if let Some(alignment) = style.text_alignment {
        let _ = write!(xml, " textAlignment=\"{}\"", h_align(alignment));
    }
    if let Some(alignment) = style.vertical_alignment {
        let _ = write!(xml, " verticalAlignment=\"{}\"", v_align(alignment));
    }
    if let Some(rotation) = style.rotation {
        let _ = write!(xml, " rotation=\"{}\"", rotation_name(rotation));
    }
    xml.push('>');
    if style.font_size.is_some()
        || style.bold
        || style.italic
        || style.underline
        || style.strike_through
        || style.font_name.is_some()
    {
        xml.push_str(&pad);
        xml.push_str("\t<font");
        if let Some(name) = &style.font_name {
            let _ = write!(xml, " fontName=\"{}\"", escape(name));
        }
        if let Some(size) = style.font_size {
            let _ = write!(xml, " size=\"{}\"", num(size));
        }
        if style.bold {
            xml.push_str(" isBold=\"true\"");
        }
        if style.italic {
            xml.push_str(" isItalic=\"true\"");
        }
        if style.underline {
            xml.push_str(" isUnderline=\"true\"");
        }
        if style.strike_through {
            xml.push_str(" isStrikeThrough=\"true\"");
        }
        xml.push_str("/>");
    }
    if style.left_indent.is_some()
        || style.right_indent.is_some()
        || style.first_line_indent.is_some()
        || style.spacing_before.is_some()
        || style.tab_stop_width.is_some()
        || !style.tab_stops.is_empty()
        || style.line_spacing.is_some()
        || style.line_spacing_size.is_some()
    {
        xml.push_str(&pad);
        xml.push_str("\t<paragraph");
        if let Some(spacing) = style.line_spacing {
            let _ = write!(xml, " lineSpacing=\"{}\"", line_spacing_name(spacing));
        }
        if let Some(size) = style.line_spacing_size {
            let _ = write!(xml, " lineSpacingSize=\"{}\"", num(size));
        }
        if let Some(indent) = style.left_indent {
            let _ = write!(xml, " leftIndent=\"{}\"", num(indent));
        }
        if let Some(indent) = style.right_indent {
            let _ = write!(xml, " rightIndent=\"{}\"", num(indent));
        }
        if let Some(indent) = style.first_line_indent {
            let _ = write!(xml, " firstLineIndent=\"{}\"", num(indent));
        }
        if let Some(spacing) = style.spacing_before {
            let _ = write!(xml, " spacingBefore=\"{}\"", num(spacing));
        }
        if let Some(width) = style.tab_stop_width {
            let _ = write!(xml, " tabStopWidth=\"{}\"", num(width));
        }
        if style.tab_stops.is_empty() {
            xml.push_str("/>");
        } else {
            xml.push_str(">\n");
            for stop in &style.tab_stops {
                let _ = writeln!(
                    xml,
                    "{pad}\t\t<tabStop position=\"{}\" alignment=\"{}\"/>",
                    num(stop.position),
                    tab_align_name(stop.alignment),
                );
            }
            let _ = writeln!(xml, "{pad}\t</paragraph>");
        }
    }
    xml.push_str(&pad);
    xml.push_str("</textElement>\n");
}

fn mode_name(mode: Mode) -> &'static str {
    match mode {
        Mode::Opaque => "Opaque",
        Mode::Transparent => "Transparent",
    }
}

fn position_name(position: PositionType) -> &'static str {
    match position {
        PositionType::FixRelativeToTop => "FixRelativeToTop",
        PositionType::Float => "Float",
        PositionType::FixRelativeToBottom => "FixRelativeToBottom",
    }
}

fn rotation_name(rotation: Rotation) -> &'static str {
    match rotation {
        Rotation::Left => "Left",
        Rotation::Right => "Right",
        Rotation::UpsideDown => "UpsideDown",
    }
}

fn markup_name(markup: Markup) -> &'static str {
    match markup {
        Markup::None => "none",
        Markup::Styled => "styled",
        Markup::Html => "html",
    }
}

fn tab_align_name(alignment: TabStopAlign) -> &'static str {
    match alignment {
        TabStopAlign::Left => "Left",
        TabStopAlign::Center => "Center",
        TabStopAlign::Right => "Right",
    }
}

fn line_spacing_name(spacing: LineSpacing) -> &'static str {
    match spacing {
        LineSpacing::Single => "Single",
        LineSpacing::OneAndHalf => "1_1_2",
        LineSpacing::Double => "Double",
        LineSpacing::Proportional => "Proportional",
        LineSpacing::AtLeast => "AtLeast",
        LineSpacing::Fixed => "Fixed",
    }
}

fn h_align(alignment: TextAlignment) -> &'static str {
    match alignment {
        TextAlignment::Left => "Left",
        TextAlignment::Center => "Center",
        TextAlignment::Right => "Right",
        TextAlignment::Justified => "Justified",
    }
}

fn v_align(alignment: VerticalAlignment) -> &'static str {
    match alignment {
        VerticalAlignment::Top => "Top",
        VerticalAlignment::Middle => "Middle",
        VerticalAlignment::Bottom => "Bottom",
    }
}

fn hex(color: Color) -> String {
    format!("#{:02X}{:02X}{:02X}", color.r, color.g, color.b)
}

fn num(value: f32) -> String {
    if value.is_finite() && value.fract() == 0.0 && value.abs() < i64::MAX as f32 {
        format!("{}", value as i64)
    } else {
        format!("{value}")
    }
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
