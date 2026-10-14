//! Read a JRXML subset into [`Report`](crate::model::Report).
//!
//! Supported tags become fields on the model. Studio properties and dataset
//! `group` tags are dropped. A JSON `queryString` is the main record path.
//! Every other tag becomes a [`Warning`](crate::Warning).

use roxmltree::Node;

use crate::expr::Expression;
use crate::model::*;
use crate::{Error, Warning};

const JSON_FIELD_EXPRESSION: &str = "net.sf.jasperreports.json.field.expression";
const STUDIO_PROPERTY_PREFIX: &str = "com.jaspersoft.studio.";

const KNOWN: &[&str] = &[
    "jasperReport",
    "style",
    "box",
    "pen",
    "topPen",
    "leftPen",
    "bottomPen",
    "rightPen",
    "field",
    "fieldDescription",
    "property",
    "background",
    "title",
    "pageHeader",
    "columnHeader",
    "detail",
    "summary",
    "pageFooter",
    "band",
    "reportElement",
    "staticText",
    "text",
    "textField",
    "textFieldExpression",
    "textElement",
    "font",
    "paragraph",
    "tabStop",
    "line",
    "rectangle",
    "ellipse",
    "image",
    "imageExpression",
    "elementGroup",
    "break",
    "graphicElement",
    "subDataset",
    "componentElement",
    "table",
    "column",
    "tableHeader",
    "detailCell",
    "columnFooter",
    "tableFooter",
    "datasetRun",
    "dataSourceExpression",
    "subreport",
    "subreportExpression",
];

/// Parse `template_xml`. Unreadable XML, or a root that is not `jasperReport`, is an error.
pub fn parse(template_xml: &str) -> Result<Report, Error> {
    let doc =
        roxmltree::Document::parse(template_xml).map_err(|err| Error::Xml(err.to_string()))?;
    let root = doc.root().first_element_child().ok_or(Error::NotAReport)?;
    if root.tag_name().name() != "jasperReport" {
        return Err(Error::NotAReport);
    }

    let mut parser = Parser::default();
    let mut report = Report {
        name: root.attribute("name").unwrap_or("").to_string(),
        page_width: parser.f32_attr(root, "pageWidth", "jasperReport"),
        page_height: parser.f32_attr(root, "pageHeight", "jasperReport"),
        column_width: parser.f32_attr(root, "columnWidth", "jasperReport"),
        left_margin: parser.f32_attr(root, "leftMargin", "jasperReport"),
        right_margin: parser.f32_attr(root, "rightMargin", "jasperReport"),
        top_margin: parser.f32_attr(root, "topMargin", "jasperReport"),
        bottom_margin: parser.f32_attr(root, "bottomMargin", "jasperReport"),
        styles: Vec::new(),
        fields: Vec::new(),
        query: None,
        subdatasets: Vec::new(),
        bands: Vec::new(),
        warnings: Vec::new(),
    };

    for child in element_children(root) {
        let name = child.tag_name().name();
        match name {
            "style" => {
                if let Some(style) = parser.parse_style(child, "style") {
                    report.styles.push(style);
                }
            }
            "subDataset" => report.subdatasets.push(parser.parse_subdataset(child)),
            "field" => report.fields.push(parser.parse_field(child, "field")),
            "queryString" => {
                let text = element_text(child);
                if !text.is_empty() {
                    report.query = Some(text);
                }
            }
            "property" => {
                parser.note_property(child, "property");
            }
            other => match BandKind::from_tag(other) {
                Some(kind) => report.bands.push(parser.parse_band(child, kind)),
                None => {
                    parser.warn(other, other);
                    parser.collect_unknown(child, other);
                }
            },
        }
    }

    report.warnings = parser.warnings;
    Ok(report)
}

#[derive(Default)]
struct Parser {
    warnings: Vec<Warning>,
}

impl Parser {
    fn warn(&mut self, tag: &str, location: &str) {
        self.warnings.push(Warning {
            tag: tag.to_string(),
            location: location.to_string(),
        });
    }

    fn f32_attr(&mut self, node: Node<'_, '_>, attr: &str, location: &str) -> f32 {
        match node.attribute(attr) {
            Some(value) => value.parse().unwrap_or_else(|_| {
                self.warn(attr, location);
                0.0
            }),
            None => 0.0,
        }
    }

    fn optional_f32(&mut self, node: Node<'_, '_>, attr: &str, location: &str) -> Option<f32> {
        let value = node.attribute(attr)?;
        match value.parse() {
            Ok(number) => Some(number),
            Err(_) => {
                self.warn(attr, location);
                None
            }
        }
    }

    fn optional_color(&mut self, node: Node<'_, '_>, attr: &str, location: &str) -> Option<Color> {
        let value = node.attribute(attr)?;
        match Color::parse(value) {
            Some(color) => Some(color),
            None => {
                self.warn(attr, location);
                None
            }
        }
    }

    fn note_property(&mut self, node: Node<'_, '_>, location: &str) -> Option<String> {
        let name = node.attribute("name").unwrap_or("");
        if name == JSON_FIELD_EXPRESSION {
            return node.attribute("value").map(str::to_string);
        }
        if name.starts_with(STUDIO_PROPERTY_PREFIX) || name.starts_with("pdfrost.") {
            return None;
        }
        self.warn("property", location);
        None
    }

    fn collect_unknown(&mut self, node: Node<'_, '_>, location: &str) {
        for child in element_children(node) {
            let name = child.tag_name().name();
            let child_location = join(location, name);
            if name == "property" {
                self.note_property(child, &child_location);
                continue;
            }
            if matches!(name, "queryString" | "group" | "groupExpression") {
                self.collect_unknown(child, &child_location);
                continue;
            }
            if KNOWN.contains(&name) {
                self.collect_unknown(child, &child_location);
                continue;
            }
            self.warn(name, &child_location);
            self.collect_unknown(child, &child_location);
        }
    }

    fn parse_style(&mut self, node: Node<'_, '_>, location: &str) -> Option<Style> {
        let name = node.attribute("name")?.to_string();
        let mut style = Style {
            name,
            mode: None,
            backcolor: None,
            borders: None,
        };
        if let Some(mode) = node.attribute("mode") {
            style.mode = self.parse_mode(mode, location);
        }
        style.backcolor = self.optional_color(node, "backcolor", location);
        for child in element_children(node) {
            let child_name = child.tag_name().name();
            let child_location = join(location, child_name);
            match child_name {
                "box" => style.borders = Some(self.parse_box(child, &child_location)),
                "property" => {
                    self.note_property(child, &child_location);
                }
                other => {
                    self.warn(other, &child_location);
                    self.collect_unknown(child, &child_location);
                }
            }
        }
        Some(style)
    }

    fn parse_mode(&mut self, value: &str, location: &str) -> Option<Mode> {
        match value {
            "Opaque" => Some(Mode::Opaque),
            "Transparent" => Some(Mode::Transparent),
            other => {
                self.warn(&format!("mode={other}"), location);
                None
            }
        }
    }

    fn parse_box(&mut self, node: Node<'_, '_>, location: &str) -> Box {
        let mut borders = Box::default();
        if let Some(padding) = self.optional_f32(node, "padding", location) {
            borders.padding_top = padding;
            borders.padding_left = padding;
            borders.padding_bottom = padding;
            borders.padding_right = padding;
        }
        if let Some(padding) = self.optional_f32(node, "topPadding", location) {
            borders.padding_top = padding;
        }
        if let Some(padding) = self.optional_f32(node, "leftPadding", location) {
            borders.padding_left = padding;
        }
        if let Some(padding) = self.optional_f32(node, "bottomPadding", location) {
            borders.padding_bottom = padding;
        }
        if let Some(padding) = self.optional_f32(node, "rightPadding", location) {
            borders.padding_right = padding;
        }
        for child in element_children(node) {
            let name = child.tag_name().name();
            let child_location = join(location, name);
            let pen = self.parse_pen(child, &child_location);
            match name {
                "pen" => borders.pen = Some(pen),
                "topPen" => borders.top = Some(pen),
                "leftPen" => borders.left = Some(pen),
                "bottomPen" => borders.bottom = Some(pen),
                "rightPen" => borders.right = Some(pen),
                other => {
                    self.warn(other, &child_location);
                    self.collect_unknown(child, &child_location);
                }
            }
        }
        borders
    }

    fn parse_pen(&mut self, node: Node<'_, '_>, location: &str) -> Pen {
        let line_style = node.attribute("lineStyle").map(str::to_string);
        if let Some(style) = line_style.as_deref() {
            if style != "Solid" {
                self.warn(&format!("lineStyle={style}"), location);
            }
        }
        for child in element_children(node) {
            let name = child.tag_name().name();
            let child_location = join(location, name);
            self.warn(name, &child_location);
            self.collect_unknown(child, &child_location);
        }
        Pen {
            line_width: self.optional_f32(node, "lineWidth", location),
            line_color: self.optional_color(node, "lineColor", location),
            line_style,
        }
    }

    fn parse_subdataset(&mut self, node: Node<'_, '_>) -> SubDataset {
        let name = node.attribute("name").unwrap_or("").to_string();
        let location = format!("subDataset[{name}]");
        let mut fields = Vec::new();
        for child in element_children(node) {
            let child_name = child.tag_name().name();
            let child_location = join(&location, child_name);
            match child_name {
                "field" => fields.push(self.parse_field(child, &child_location)),
                "queryString" => {}
                "group" => self.ignore_group(child, &child_location),
                "property" => {
                    self.note_property(child, &child_location);
                }
                other => {
                    self.warn(other, &child_location);
                    self.collect_unknown(child, &child_location);
                }
            }
        }
        SubDataset { name, fields }
    }

    fn ignore_group(&mut self, node: Node<'_, '_>, location: &str) {
        for child in element_children(node) {
            let name = child.tag_name().name();
            if name == "groupExpression" {
                continue;
            }
            let child_location = join(location, name);
            self.warn(name, &child_location);
            self.collect_unknown(child, &child_location);
        }
    }

    fn parse_field(&mut self, node: Node<'_, '_>, location: &str) -> Field {
        let mut json_path = None;
        let mut description = None;
        for child in element_children(node) {
            let name = child.tag_name().name();
            let child_location = join(location, name);
            match name {
                "property" => {
                    if let Some(path) = self.note_property(child, &child_location) {
                        json_path = Some(path);
                    }
                }
                "fieldDescription" => description = Some(element_text(child)),
                other => {
                    self.warn(other, &child_location);
                    self.collect_unknown(child, &child_location);
                }
            }
        }
        Field {
            name: node.attribute("name").unwrap_or("").to_string(),
            class_name: node
                .attribute("class")
                .unwrap_or("java.lang.String")
                .to_string(),
            description,
            json_path,
        }
    }

    fn parse_band(&mut self, node: Node<'_, '_>, kind: BandKind) -> Band {
        let location = kind.name();
        let mut band = Band {
            kind,
            height: 0.0,
            split_type: SplitType::Stretch,
            elements: Vec::new(),
        };
        let mut saw_band = false;
        for child in element_children(node) {
            let name = child.tag_name().name();
            let child_location = join(location, name);
            if name == "band" {
                saw_band = true;
                band.height = self.f32_attr(child, "height", &child_location);
                if let Some(split) = child.attribute("splitType") {
                    if split != "Stretch" {
                        self.warn(&format!("splitType={split}"), &child_location);
                    }
                }
                band.elements = self.parse_element_children(child, location);
            } else {
                self.warn(name, &child_location);
                self.collect_unknown(child, &child_location);
            }
        }
        if !saw_band {
            self.warn("band", location);
        }
        band
    }

    fn parse_element_children(&mut self, node: Node<'_, '_>, location: &str) -> Vec<Element> {
        let mut elements = Vec::new();
        for child in element_children(node) {
            let name = child.tag_name().name();
            let child_location = join(location, name);
            match name {
                "staticText" => elements.push(Element::StaticText(
                    self.parse_static_text(child, &child_location),
                )),
                "textField" => elements.push(Element::TextField(
                    self.parse_text_field(child, &child_location),
                )),
                "line" => elements.push(Element::Line(Graphic {
                    element: self.parse_graphic(child, &child_location),
                })),
                "rectangle" => elements.push(Element::Rectangle(Graphic {
                    element: self.parse_graphic(child, &child_location),
                })),
                "ellipse" => elements.push(Element::Ellipse(Graphic {
                    element: self.parse_graphic(child, &child_location),
                })),
                "image" => elements.push(Element::Image(self.parse_image(child, &child_location))),
                "break" => elements.push(self.parse_break(child, &child_location)),
                "elementGroup" => {
                    elements.extend(self.parse_element_children(child, &child_location));
                }
                name if is_chart_tag(name) => {
                    elements.push(Element::Chart(self.parse_chart(child, &child_location)));
                }
                "subreport" => {
                    elements.push(Element::Subreport(
                        self.parse_subreport(child, &child_location),
                    ));
                }
                "componentElement" => {
                    if let Some(table) = self.parse_component(child, &child_location) {
                        elements.push(Element::Table(table));
                    }
                }
                "property" => {
                    self.note_property(child, &child_location);
                }
                other => {
                    self.warn(other, &child_location);
                    self.collect_unknown(child, &child_location);
                }
            }
        }
        elements
    }

    fn parse_subreport(&mut self, node: Node<'_, '_>, location: &str) -> Subreport {
        let mut element = ReportElement::default();
        let mut expression = String::new();
        let mut data_source = None;
        let mut saw_frame = false;
        for child in element_children(node) {
            let name = child.tag_name().name();
            let child_location = join(location, name);
            match name {
                "reportElement" if !saw_frame => {
                    element = self.parse_report_element(child, &child_location);
                    saw_frame = true;
                }
                "box" => element.borders = Some(self.parse_box(child, &child_location)),
                "subreportExpression" => expression = element_text(child),
                "dataSourceExpression" => {
                    let text = element_text(child);
                    if let Some(source) = sub_data_source(&text) {
                        data_source = Some(source);
                    } else if !text.is_empty() && !bare_sub_data_source(&text) {
                        self.warn(&text, &child_location);
                    }
                }
                "connectionExpression"
                | "subreportParameter"
                | "returnValue"
                | "parametersMapExpression" => {
                    self.warn(name, &child_location);
                }
                "property" => {
                    self.note_property(child, &child_location);
                }
                other => {
                    self.warn(other, &child_location);
                    self.collect_unknown(child, &child_location);
                }
            }
        }
        if !expression.is_empty() {
            match Expression::parse(&expression) {
                Expression::Text(_) | Expression::Field(_) => {}
                _ => self.warn(&expression, location),
            }
        }
        Subreport {
            element,
            expression,
            data_source,
        }
    }

    fn parse_static_text(&mut self, node: Node<'_, '_>, location: &str) -> StaticText {
        let (element, text_style, text) = self.parse_text_container(node, location, "text");
        StaticText {
            element,
            text_style,
            text,
            stretch: node.attribute("isStretchWithOverflow") == Some("true"),
        }
    }

    fn parse_text_field(&mut self, node: Node<'_, '_>, location: &str) -> TextField {
        let (element, text_style, expression) =
            self.parse_text_container(node, location, "textFieldExpression");
        if !supported_expression(&expression) {
            let tag = if expression.is_empty() {
                "textFieldExpression".to_string()
            } else {
                expression.clone()
            };
            self.warn(&tag, location);
        }
        TextField {
            element,
            text_style,
            expression,
            stretch: node.attribute("isStretchWithOverflow") == Some("true"),
        }
    }

    fn parse_text_container(
        &mut self,
        node: Node<'_, '_>,
        location: &str,
        text_tag: &str,
    ) -> (ReportElement, TextStyle, String) {
        let mut element = ReportElement::default();
        let mut text_style = TextStyle::default();
        let mut text = String::new();
        let mut saw_frame = false;
        for child in element_children(node) {
            let name = child.tag_name().name();
            let child_location = join(location, name);
            match name {
                "reportElement" if !saw_frame => {
                    element = self.parse_report_element(child, &child_location);
                    saw_frame = true;
                }
                "box" => element.borders = Some(self.parse_box(child, &child_location)),
                "textElement" => text_style = self.parse_text_style(child, &child_location),
                name if name == text_tag => text = element_text(child),
                "property" => {
                    self.note_property(child, &child_location);
                }
                other => {
                    self.warn(other, &child_location);
                    self.collect_unknown(child, &child_location);
                }
            }
        }
        (element, text_style, text)
    }

    fn parse_chart(&mut self, node: Node<'_, '_>, location: &str) -> Chart {
        let kind = node.tag_name().name().to_string();
        let mut chart = Chart {
            element: ReportElement::default(),
            kind,
            evaluation_time: None,
            title: None,
            subtitle: None,
            show_legend: true,
            data_source: None,
            sub_dataset: None,
            series: Vec::new(),
        };
        self.read_chart(node, location, &mut chart);
        if !supported_chart(&chart.kind) {
            self.warn("chart", &join(location, "chart"));
        }
        chart
    }

    fn read_chart(&mut self, node: Node<'_, '_>, location: &str, chart: &mut Chart) {
        for child in element_children(node) {
            let name = child.tag_name().name();
            let child_location = join(location, name);
            match name {
                "chart" => {
                    self.read_chart_attributes(child, &child_location, chart);
                    self.read_chart(child, &child_location, chart);
                }
                "reportElement" => {
                    chart.element = self.parse_report_element(child, &child_location);
                }
                "box" => chart.element.borders = Some(self.parse_box(child, &child_location)),
                "chartTitle" => {
                    chart.title = child_expression(child, "titleExpression");
                    self.skip_known_chart_children(
                        child,
                        &child_location,
                        &["titleExpression", "font"],
                    );
                }
                "chartSubtitle" => {
                    chart.subtitle = child_expression(child, "subtitleExpression");
                    self.skip_known_chart_children(
                        child,
                        &child_location,
                        &["subtitleExpression", "font"],
                    );
                }
                "chartLegend" => {}
                "categoryDataset" | "dataset" | "areaPlot" | "barPlot" | "linePlot" | "plot"
                | "categoryAxisFormat" | "valueAxisFormat" | "axisFormat" => {
                    self.read_chart(child, &child_location, chart);
                }
                "categorySeries" => {
                    chart
                        .series
                        .push(self.parse_category_series(child, &child_location));
                }
                "datasetRun" => {
                    let (sub_dataset, data_source) = self.parse_dataset_run(child, &child_location);
                    if sub_dataset.is_some() {
                        chart.sub_dataset = sub_dataset;
                    }
                    if data_source.is_some() {
                        chart.data_source = data_source;
                    }
                }
                "property" => {
                    self.note_property(child, &child_location);
                }
                "seriesColor" | "font" | "labelFont" | "tickLabelFont" => {}
                other => {
                    self.warn(other, &child_location);
                    self.collect_unknown(child, &child_location);
                }
            }
        }
    }

    fn read_chart_attributes(&mut self, node: Node<'_, '_>, location: &str, chart: &mut Chart) {
        if let Some(time) = node.attribute("evaluationTime") {
            chart.evaluation_time = Some(time.to_string());
            if time != "Now" && time != "Report" {
                self.warn(&format!("evaluationTime={time}"), location);
            }
        }
        if let Some(flag) = node.attribute("isShowLegend") {
            match flag {
                "true" => chart.show_legend = true,
                "false" => chart.show_legend = false,
                other => self.warn(&format!("isShowLegend={other}"), location),
            }
        }
        for attr in ["customizerClass", "renderType", "theme", "evaluationGroup"] {
            if node.attribute(attr).is_some() {
                self.warn(attr, location);
            }
        }
    }

    fn parse_category_series(&mut self, node: Node<'_, '_>, location: &str) -> CategorySeries {
        let mut series = CategorySeries::default();
        for child in element_children(node) {
            let name = child.tag_name().name();
            let child_location = join(location, name);
            match name {
                "seriesExpression" => series.series_expression = element_text(child),
                "categoryExpression" => series.category_expression = element_text(child),
                "valueExpression" => series.value_expression = element_text(child),
                "property" => {
                    self.note_property(child, &child_location);
                }
                other => {
                    self.warn(other, &child_location);
                    self.collect_unknown(child, &child_location);
                }
            }
        }
        series
    }

    fn skip_known_chart_children(&mut self, node: Node<'_, '_>, location: &str, allowed: &[&str]) {
        for child in element_children(node) {
            let name = child.tag_name().name();
            let child_location = join(location, name);
            if name == "property" {
                self.note_property(child, &child_location);
            } else if !allowed.contains(&name) {
                self.warn(name, &child_location);
                self.collect_unknown(child, &child_location);
            }
        }
    }

    fn parse_graphic(&mut self, node: Node<'_, '_>, location: &str) -> ReportElement {
        let mut element = ReportElement::default();
        let mut saw_frame = false;
        for child in element_children(node) {
            let name = child.tag_name().name();
            let child_location = join(location, name);
            match name {
                "reportElement" if !saw_frame => {
                    element = self.parse_report_element(child, &child_location);
                    saw_frame = true;
                }
                "box" => element.borders = Some(self.parse_box(child, &child_location)),
                "graphicElement" => {
                    if let Some(pen) = self.graphic_pen(child, &child_location) {
                        let borders = element.borders.get_or_insert(Box::default());
                        if borders.pen.is_none() {
                            borders.pen = Some(pen);
                        }
                    }
                }
                "property" => {
                    self.note_property(child, &child_location);
                }
                other => {
                    self.warn(other, &child_location);
                    self.collect_unknown(child, &child_location);
                }
            }
        }
        element
    }

    fn graphic_pen(&mut self, node: Node<'_, '_>, location: &str) -> Option<Pen> {
        let mut pen = None;
        for child in element_children(node) {
            let name = child.tag_name().name();
            let child_location = join(location, name);
            match name {
                "pen" => pen = Some(self.parse_pen(child, &child_location)),
                "property" => {
                    self.note_property(child, &child_location);
                }
                other => {
                    self.warn(other, &child_location);
                    self.collect_unknown(child, &child_location);
                }
            }
        }
        pen
    }

    fn parse_image(&mut self, node: Node<'_, '_>, location: &str) -> crate::model::Image {
        for attr in node.attributes() {
            if !matches!(
                attr.name(),
                "scaleImage"
                    | "rotation"
                    | "hAlign"
                    | "vAlign"
                    | "isUsingCache"
                    | "isLazy"
                    | "onErrorType"
                    | "evaluationTime"
                    | "evaluationGroup"
            ) {
                self.warn(attr.name(), location);
            }
        }
        let mut image = crate::model::Image {
            scale_image: node
                .attribute("scaleImage")
                .and_then(|value| self.parse_scale_image(value, location)),
            h_align: node
                .attribute("hAlign")
                .and_then(|value| self.parse_image_h_align(value, location)),
            v_align: node
                .attribute("vAlign")
                .and_then(|value| self.parse_vertical_alignment(value, location)),
            rotation: node
                .attribute("rotation")
                .and_then(|value| self.parse_rotation(value, location)),
            using_cache: self.optional_bool(node, "isUsingCache", location),
            lazy: node.attribute("isLazy") == Some("true"),
            on_error_type: node
                .attribute("onErrorType")
                .and_then(|value| self.parse_on_error(value, location)),
            evaluation_time: node.attribute("evaluationTime").map(str::to_string),
            evaluation_group: node.attribute("evaluationGroup").map(str::to_string),
            ..crate::model::Image::default()
        };
        if let Some(flag) = node.attribute("isLazy") {
            if flag != "true" && flag != "false" {
                self.warn(&format!("isLazy={flag}"), location);
            }
        }
        let mut saw_frame = false;
        for child in element_children(node) {
            let name = child.tag_name().name();
            let child_location = join(location, name);
            match name {
                "reportElement" if !saw_frame => {
                    image.pictures = Self::pdfrost_pictures(child);
                    image.element = self.parse_report_element(child, &child_location);
                    saw_frame = true;
                }
                "box" => image.element.borders = Some(self.parse_box(child, &child_location)),
                "graphicElement" => {
                    if let Some(pen) = self.graphic_pen(child, &child_location) {
                        let borders = image.element.borders.get_or_insert(Box::default());
                        if borders.pen.is_none() {
                            borders.pen = Some(pen);
                        }
                    }
                }
                "imageExpression" => {
                    image.expression = element_text(child);
                    if let Some(class) = child.attribute("class") {
                        if class != "java.lang.String" {
                            image.expression_class = Some(class.to_string());
                        }
                    }
                }
                "property" => {
                    self.note_property(child, &child_location);
                }
                other => {
                    self.warn(other, &child_location);
                    self.collect_unknown(child, &child_location);
                }
            }
        }
        image
    }

    fn parse_scale_image(&mut self, value: &str, location: &str) -> Option<ScaleImage> {
        match value {
            "Clip" => Some(ScaleImage::Clip),
            "FillFrame" => Some(ScaleImage::FillFrame),
            "RetainShape" => Some(ScaleImage::RetainShape),
            "RealHeight" => Some(ScaleImage::RealHeight),
            "RealSize" => Some(ScaleImage::RealSize),
            other => {
                self.warn(&format!("scaleImage={other}"), location);
                None
            }
        }
    }

    fn parse_image_h_align(&mut self, value: &str, location: &str) -> Option<HorizontalImageAlign> {
        match value {
            "Left" => Some(HorizontalImageAlign::Left),
            "Center" => Some(HorizontalImageAlign::Center),
            "Right" => Some(HorizontalImageAlign::Right),
            other => {
                self.warn(&format!("hAlign={other}"), location);
                None
            }
        }
    }

    fn parse_on_error(&mut self, value: &str, location: &str) -> Option<OnErrorType> {
        match value {
            "Error" => Some(OnErrorType::Error),
            "Blank" => Some(OnErrorType::Blank),
            "Icon" => Some(OnErrorType::Icon),
            other => {
                self.warn(&format!("onErrorType={other}"), location);
                None
            }
        }
    }

    fn optional_bool(&mut self, node: Node<'_, '_>, name: &str, location: &str) -> Option<bool> {
        match node.attribute(name) {
            Some("true") => Some(true),
            Some("false") => Some(false),
            Some(other) => {
                self.warn(&format!("{name}={other}"), location);
                None
            }
            None => None,
        }
    }

    fn parse_break(&mut self, node: Node<'_, '_>, location: &str) -> Element {
        let column = match node.attribute("type") {
            Some("Column") => true,
            Some("Page") | None => false,
            Some(other) => {
                self.warn(&format!("type={other}"), location);
                false
            }
        };
        Element::Break(PageBreak {
            element: self.parse_graphic(node, location),
            column,
        })
    }

fn pdfrost_pictures(node: Node<'_, '_>) -> std::collections::BTreeMap<String, String> {
    let mut pictures = std::collections::BTreeMap::new();
    for child in element_children(node) {
        if child.tag_name().name() != "property" {
            continue;
        }
        let Some(name) = child.attribute("name") else {
            continue;
        };
        if !name.starts_with("pdfrost.") {
            continue;
        }
        let Some(value) = child.attribute("value") else {
            continue;
        };
        if !value.is_empty() {
            pictures.insert(name.to_string(), value.to_string());
        }
    }
    pictures
}

    fn parse_report_element(&mut self, node: Node<'_, '_>, location: &str) -> ReportElement {
        let mut element = ReportElement {
            x: self.f32_attr(node, "x", location),
            y: self.f32_attr(node, "y", location),
            width: self.f32_attr(node, "width", location),
            height: self.f32_attr(node, "height", location),
            ..ReportElement::default()
        };
        if let Some(mode) = node.attribute("mode") {
            element.mode = self.parse_mode(mode, location);
        }
        element.backcolor = self.optional_color(node, "backcolor", location);
        if let Some(position) = node.attribute("positionType") {
            element.position = self.parse_position(position, location);
        }
        for child in element_children(node) {
            let name = child.tag_name().name();
            let child_location = join(location, name);
            match name {
                "property" => {
                    self.note_property(child, &child_location);
                }
                other => {
                    self.warn(other, &child_location);
                    self.collect_unknown(child, &child_location);
                }
            }
        }
        element
    }

    fn parse_text_style(&mut self, node: Node<'_, '_>, location: &str) -> TextStyle {
        let mut style = TextStyle::default();
        if let Some(alignment) = node.attribute("textAlignment") {
            style.text_alignment = self.parse_text_alignment(alignment, location);
        }
        if let Some(alignment) = node.attribute("verticalAlignment") {
            style.vertical_alignment = self.parse_vertical_alignment(alignment, location);
        }
        if let Some(rotation) = node.attribute("rotation") {
            style.rotation = self.parse_rotation(rotation, location);
        }
        if let Some(markup) = node.attribute("markup") {
            style.markup = self.parse_markup(markup, location);
        }
        for child in element_children(node) {
            let name = child.tag_name().name();
            let child_location = join(location, name);
            match name {
                "font" => {
                    style.font_size = self.optional_f32(child, "size", &child_location);
                    style.bold = child.attribute("isBold") == Some("true");
                    style.italic = child.attribute("isItalic") == Some("true");
                    style.underline = child.attribute("isUnderline") == Some("true");
                    style.strike_through = child.attribute("isStrikeThrough") == Some("true");
                    if let Some(name) = child.attribute("fontName") {
                        if !known_sans(name) {
                            self.warn(&format!("fontName={name}"), &child_location);
                        }
                        style.font_name = Some(name.to_string());
                    }
                }
                "paragraph" => {
                    style.left_indent = self.optional_f32(child, "leftIndent", &child_location);
                    style.right_indent = self.optional_f32(child, "rightIndent", &child_location);
                    style.first_line_indent =
                        self.optional_f32(child, "firstLineIndent", &child_location);
                    style.spacing_before =
                        self.optional_f32(child, "spacingBefore", &child_location);
                    style.tab_stop_width =
                        self.optional_f32(child, "tabStopWidth", &child_location);
                    if let Some(spacing) = child.attribute("lineSpacing") {
                        style.line_spacing = self.parse_line_spacing(spacing, &child_location);
                    }
                    style.line_spacing_size =
                        self.optional_f32(child, "lineSpacingSize", &child_location);
                    for stop in element_children(child) {
                        let stop_name = stop.tag_name().name();
                        let stop_location = join(&child_location, stop_name);
                        if stop_name == "tabStop" {
                            let position = self.f32_attr(stop, "position", &stop_location);
                            let alignment = match stop.attribute("alignment") {
                                Some("Center") => TabStopAlign::Center,
                                Some("Right") => TabStopAlign::Right,
                                Some("Left") | None => TabStopAlign::Left,
                                Some(other) => {
                                    self.warn(&format!("alignment={other}"), &stop_location);
                                    TabStopAlign::Left
                                }
                            };
                            style.tab_stops.push(TabStop {
                                position,
                                alignment,
                            });
                        } else {
                            self.warn(stop_name, &stop_location);
                        }
                    }
                }
                "property" => {
                    self.note_property(child, &child_location);
                }
                other => {
                    self.warn(other, &child_location);
                    self.collect_unknown(child, &child_location);
                }
            }
        }
        style
    }

    fn parse_text_alignment(&mut self, value: &str, location: &str) -> Option<TextAlignment> {
        match value {
            "Left" => Some(TextAlignment::Left),
            "Center" => Some(TextAlignment::Center),
            "Right" => Some(TextAlignment::Right),
            "Justified" => Some(TextAlignment::Justified),
            other => {
                self.warn(&format!("textAlignment={other}"), location);
                None
            }
        }
    }

    fn parse_markup(&mut self, value: &str, location: &str) -> Markup {
        match value {
            "none" => Markup::None,
            "styled" => Markup::Styled,
            "html" => Markup::Html,
            other => {
                self.warn(&format!("markup={other}"), location);
                Markup::None
            }
        }
    }

    fn parse_rotation(&mut self, value: &str, location: &str) -> Option<Rotation> {
        match value {
            "None" => None,
            "Left" => Some(Rotation::Left),
            "Right" => Some(Rotation::Right),
            "UpsideDown" => Some(Rotation::UpsideDown),
            other => {
                self.warn(&format!("rotation={other}"), location);
                None
            }
        }
    }

    fn parse_line_spacing(&mut self, value: &str, location: &str) -> Option<LineSpacing> {
        match value {
            "Single" => Some(LineSpacing::Single),
            "1_1_2" => Some(LineSpacing::OneAndHalf),
            "Double" => Some(LineSpacing::Double),
            "Proportional" => Some(LineSpacing::Proportional),
            "AtLeast" => Some(LineSpacing::AtLeast),
            "Fixed" => Some(LineSpacing::Fixed),
            other => {
                self.warn(&format!("lineSpacing={other}"), location);
                None
            }
        }
    }

    fn parse_position(&mut self, value: &str, location: &str) -> PositionType {
        match value {
            "FixRelativeToTop" => PositionType::FixRelativeToTop,
            "Float" => PositionType::Float,
            "FixRelativeToBottom" => PositionType::FixRelativeToBottom,
            other => {
                self.warn(&format!("positionType={other}"), location);
                PositionType::FixRelativeToTop
            }
        }
    }

    fn parse_vertical_alignment(
        &mut self,
        value: &str,
        location: &str,
    ) -> Option<VerticalAlignment> {
        match value {
            "Top" => Some(VerticalAlignment::Top),
            "Middle" => Some(VerticalAlignment::Middle),
            "Bottom" => Some(VerticalAlignment::Bottom),
            other => {
                self.warn(&format!("verticalAlignment={other}"), location);
                None
            }
        }
    }

    fn parse_component(&mut self, node: Node<'_, '_>, location: &str) -> Option<Table> {
        let mut frame = None;
        let mut table = None;
        for child in element_children(node) {
            let name = child.tag_name().name();
            let child_location = join(location, name);
            match name {
                "reportElement" => frame = Some(self.parse_report_element(child, &child_location)),
                "table" => table = Some(self.parse_table(child, &child_location)),
                "property" => {
                    self.note_property(child, &child_location);
                }
                other => {
                    self.warn(other, &child_location);
                    self.collect_unknown(child, &child_location);
                }
            }
        }
        let mut table = table?;
        if let Some(frame) = frame {
            table.element = frame;
        }
        Some(table)
    }

    fn parse_table(&mut self, node: Node<'_, '_>, location: &str) -> Table {
        let mut table = Table {
            element: ReportElement::default(),
            when_no_data: node.attribute("whenNoDataType").map(str::to_string),
            sub_dataset: None,
            data_source: None,
            columns: Vec::new(),
        };
        for child in element_children(node) {
            let name = child.tag_name().name();
            let child_location = join(location, name);
            match name {
                "datasetRun" => {
                    let (sub_dataset, data_source) = self.parse_dataset_run(child, &child_location);
                    table.sub_dataset = sub_dataset;
                    table.data_source = data_source;
                }
                "column" => table
                    .columns
                    .push(self.parse_column(child, &child_location)),
                "property" => {
                    self.note_property(child, &child_location);
                }
                other => {
                    self.warn(other, &child_location);
                    self.collect_unknown(child, &child_location);
                }
            }
        }
        table
    }

    fn parse_dataset_run(
        &mut self,
        node: Node<'_, '_>,
        location: &str,
    ) -> (Option<String>, Option<String>) {
        let sub_dataset = node.attribute("subDataset").map(str::to_string);
        let mut data_source = None;
        for child in element_children(node) {
            let name = child.tag_name().name();
            let child_location = join(location, name);
            match name {
                "dataSourceExpression" => {
                    let expression = element_text(child);
                    match sub_data_source(&expression) {
                        Some(source) => data_source = Some(source),
                        None => {
                            let tag = if expression.is_empty() {
                                "dataSourceExpression"
                            } else {
                                &expression
                            };
                            self.warn(tag, &child_location);
                        }
                    }
                }
                other => {
                    self.warn(other, &child_location);
                    self.collect_unknown(child, &child_location);
                }
            }
        }
        (sub_dataset, data_source)
    }

    fn parse_column(&mut self, node: Node<'_, '_>, location: &str) -> Column {
        let mut column = Column {
            width: self.f32_attr(node, "width", location),
            sections: Vec::new(),
        };
        for child in element_children(node) {
            let name = child.tag_name().name();
            let child_location = join(location, name);
            if let Some(kind) = section_kind(name) {
                column.sections.push(ColumnSection {
                    kind,
                    style: child.attribute("style").map(str::to_string),
                    height: self.f32_attr(child, "height", &child_location),
                    elements: self.parse_element_children(child, &child_location),
                });
            } else if name == "property" {
                self.note_property(child, &child_location);
            } else {
                self.warn(name, &child_location);
                self.collect_unknown(child, &child_location);
            }
        }
        column
    }
}

fn is_chart_tag(name: &str) -> bool {
    name.len() > "Chart".len() && name.ends_with("Chart")
}

fn supported_chart(kind: &str) -> bool {
    matches!(
        kind,
        "areaChart" | "barChart" | "lineChart" | "stackedAreaChart" | "stackedBarChart"
    )
}

fn section_kind(name: &str) -> Option<SectionKind> {
    Some(match name {
        "tableHeader" => SectionKind::TableHeader,
        "columnHeader" => SectionKind::ColumnHeader,
        "detailCell" => SectionKind::DetailCell,
        "columnFooter" => SectionKind::ColumnFooter,
        "tableFooter" => SectionKind::TableFooter,
        _ => return None,
    })
}

fn bare_sub_data_source(expression: &str) -> bool {
    let Some(start) = expression.find("subDataSource") else {
        return false;
    };
    expression[start + "subDataSource".len()..].trim() == "()"
}

fn sub_data_source(expression: &str) -> Option<String> {
    let start = expression.find("subDataSource")?;
    let mut rest = expression[start + "subDataSource".len()..].trim_start();
    rest = rest.strip_prefix('(')?.trim_start();
    let quote = rest.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let body = &rest[quote.len_utf8()..];
    let end = body.find(quote)?;
    let name = &body[..end];
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

/// Names that share Helvetica metrics. Anything else is stored and warned; the PDF still uses Liberation Sans.
fn known_sans(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "helvetica" | "sansserif" | "arial" | "liberation sans" | "dejavu sans" | "sans-serif"
    )
}

fn supported_expression(expression: &str) -> bool {
    Expression::parse(expression).is_text_field()
}

fn element_children<'a, 'input>(node: Node<'a, 'input>) -> impl Iterator<Item = Node<'a, 'input>> {
    node.children().filter(|child| child.is_element())
}

fn element_text(node: Node<'_, '_>) -> String {
    node.text().unwrap_or("").trim().to_string()
}

fn child_expression(node: Node<'_, '_>, tag: &str) -> Option<String> {
    element_children(node)
        .find(|child| child.tag_name().name() == tag)
        .map(element_text)
}

fn join(parent: &str, child: &str) -> String {
    if parent.is_empty() {
        child.to_string()
    } else {
        format!("{parent}/{child}")
    }
}
