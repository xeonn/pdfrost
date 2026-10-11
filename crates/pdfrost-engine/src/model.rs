//! Typed JRXML. Later phases fill and draw these values; they do not re-read the XML.

use serde::{Deserialize, Serialize};

/// A parsed template and the warnings found while reading it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub name: String,
    pub page_width: f32,
    pub page_height: f32,
    pub column_width: f32,
    pub left_margin: f32,
    pub right_margin: f32,
    pub top_margin: f32,
    pub bottom_margin: f32,
    pub styles: Vec<Style>,
    pub fields: Vec<Field>,
    /// Subdataset fields stay here. They are not copied onto [`Report::fields`].
    pub subdatasets: Vec<SubDataset>,
    /// Bands in document order.
    pub bands: Vec<Band>,
    pub warnings: Vec<crate::Warning>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Style {
    pub name: String,
    pub mode: Option<Mode>,
    pub backcolor: Option<Color>,
    pub borders: Option<Box>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Mode {
    Opaque,
    Transparent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub fn parse(value: &str) -> Option<Self> {
        let hex = value.strip_prefix('#')?;
        let (r, g, b) = match hex.len() {
            6 => (
                u8::from_str_radix(&hex[0..2], 16).ok()?,
                u8::from_str_radix(&hex[2..4], 16).ok()?,
                u8::from_str_radix(&hex[4..6], 16).ok()?,
            ),
            3 => {
                let digit = |index: usize| {
                    let byte = hex.as_bytes()[index];
                    let n = match byte {
                        b'0'..=b'9' => byte - b'0',
                        b'a'..=b'f' => byte - b'a' + 10,
                        b'A'..=b'F' => byte - b'A' + 10,
                        _ => return None,
                    };
                    Some(n * 16 + n)
                };
                (digit(0)?, digit(1)?, digit(2)?)
            }
            _ => return None,
        };
        Some(Self { r, g, b })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pen {
    pub line_width: Option<f32>,
    pub line_color: Option<Color>,
    /// `Solid` when the template says so. Any other value is also stored, and parse adds a warning.
    pub line_style: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Box {
    pub pen: Option<Pen>,
    pub top: Option<Pen>,
    pub left: Option<Pen>,
    pub bottom: Option<Pen>,
    pub right: Option<Pen>,
    /// Insets from `<box padding>` and the side padding attributes. Text is laid out inside them.
    #[serde(default)]
    pub padding_top: f32,
    #[serde(default)]
    pub padding_left: f32,
    #[serde(default)]
    pub padding_bottom: f32,
    #[serde(default)]
    pub padding_right: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Field {
    pub name: String,
    /// Java class name from the template, such as `java.lang.Integer`.
    pub class_name: String,
    pub description: Option<String>,
    /// `net.sf.jasperreports.json.field.expression`, when the field has one.
    pub json_path: Option<String>,
}

impl Field {
    pub fn is_integer(&self) -> bool {
        self.class_name == "java.lang.Integer"
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubDataset {
    pub name: String,
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BandKind {
    Background,
    Title,
    PageHeader,
    ColumnHeader,
    Detail,
    ColumnFooter,
    Summary,
    PageFooter,
}

impl BandKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Background => "background",
            Self::Title => "title",
            Self::PageHeader => "pageHeader",
            Self::ColumnHeader => "columnHeader",
            Self::Detail => "detail",
            Self::ColumnFooter => "columnFooter",
            Self::Summary => "summary",
            Self::PageFooter => "pageFooter",
        }
    }

    pub(crate) fn from_tag(tag: &str) -> Option<Self> {
        Some(match tag {
            "background" => Self::Background,
            "title" => Self::Title,
            "pageHeader" => Self::PageHeader,
            "columnHeader" => Self::ColumnHeader,
            "detail" => Self::Detail,
            "columnFooter" => Self::ColumnFooter,
            "summary" => Self::Summary,
            "pageFooter" => Self::PageFooter,
            _ => return None,
        })
    }
}

/// Every band in the samples is `Stretch`. A different value is kept as `Stretch` and warned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SplitType {
    Stretch,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Band {
    pub kind: BandKind,
    pub height: f32,
    pub split_type: SplitType,
    /// Paint order is this vector's order.
    pub elements: Vec<Element>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportElement {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub mode: Option<Mode>,
    pub backcolor: Option<Color>,
    pub borders: Option<Box>,
    /// `positionType`. The default keeps the template `y`. `Float` moves down when an element above stretches.
    #[serde(default)]
    pub position: PositionType,
}

/// `positionType` on `reportElement`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PositionType {
    #[default]
    FixRelativeToTop,
    Float,
    FixRelativeToBottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TextAlignment {
    Left,
    Center,
    Right,
    Justified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VerticalAlignment {
    Top,
    Middle,
    Bottom,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextStyle {
    pub text_alignment: Option<TextAlignment>,
    pub vertical_alignment: Option<VerticalAlignment>,
    pub font_size: Option<f32>,
    pub bold: bool,
    #[serde(default)]
    pub italic: bool,
    #[serde(default)]
    pub underline: bool,
    /// `font isStrikeThrough`. A line is drawn through each run.
    #[serde(default)]
    pub strike_through: bool,
    #[serde(default)]
    pub font_name: Option<String>,
    #[serde(default)]
    pub line_spacing: Option<LineSpacing>,
    #[serde(default)]
    pub line_spacing_size: Option<f32>,
    #[serde(default)]
    pub rotation: Option<Rotation>,
    /// `textElement markup`. `None` leaves `<` as text.
    #[serde(default)]
    pub markup: Markup,
    pub left_indent: Option<f32>,
    pub right_indent: Option<f32>,
    /// Extra inset on the first line of each paragraph.
    #[serde(default)]
    pub first_line_indent: Option<f32>,
    /// Space added above the first line, in points.
    #[serde(default)]
    pub spacing_before: Option<f32>,
    /// Distance between default tab stops. JasperReports uses 40 when this is absent.
    #[serde(default)]
    pub tab_stop_width: Option<f32>,
    #[serde(default)]
    pub tab_stops: Vec<TabStop>,
}

/// `textElement markup`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Markup {
    #[default]
    None,
    Styled,
    Html,
}

/// One `paragraph/tabStop`. Positions are from the left edge of the element.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TabStop {
    pub position: f32,
    pub alignment: TabStopAlign,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TabStopAlign {
    #[default]
    Left,
    Center,
    Right,
}

/// `paragraph lineSpacing`. The first line stays single. Later lines use the named spacing.
/// The distance comes from the font's ascent, descent, and leading.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LineSpacing {
    Single,
    OneAndHalf,
    Double,
    Proportional,
    AtLeast,
    Fixed,
}

/// `textElement rotation`. `None` stays unset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Rotation {
    Left,
    Right,
    UpsideDown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StaticText {
    pub element: ReportElement,
    pub text_style: TextStyle,
    pub text: String,
    /// `isStretchWithOverflow`. The box grows to the wrapped text, and the band grows with it.
    #[serde(default)]
    pub stretch: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextField {
    pub element: ReportElement,
    pub text_style: TextStyle,
    pub expression: String,
    /// `isStretchWithOverflow`. The box grows to the wrapped text, and the band grows with it.
    #[serde(default)]
    pub stretch: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Graphic {
    pub element: ReportElement,
}

/// `scaleImage` on `<image>`. Absent means `RetainShape`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ScaleImage {
    Clip,
    FillFrame,
    RetainShape,
    RealHeight,
    RealSize,
}

/// `hAlign` on `<image>`. Absent means `Left`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HorizontalImageAlign {
    Left,
    Center,
    Right,
}

/// `onErrorType` on `<image>`. Absent means `Error`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OnErrorType {
    Error,
    Blank,
    Icon,
}

/// An `<image>`. The picture is the value of `imageExpression`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Image {
    pub element: ReportElement,
    /// Raw `imageExpression` text, including the quotes of a string literal.
    #[serde(default)]
    pub expression: String,
    /// `imageExpression class` when it is not `java.lang.String`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expression_class: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale_image: Option<ScaleImage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub h_align: Option<HorizontalImageAlign>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub v_align: Option<VerticalAlignment>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotation: Option<Rotation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub using_cache: Option<bool>,
    #[serde(default)]
    pub lazy: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_error_type: Option<OnErrorType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evaluation_time: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evaluation_group: Option<String>,
}

/// A `<break>`. `column` is `type="Column"`. Page and column breaks both start a new page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageBreak {
    pub element: ReportElement,
    pub column: bool,
}

/// One `<categorySeries>`. The strings are the raw expression text.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CategorySeries {
    pub series_expression: String,
    pub category_expression: String,
    pub value_expression: String,
}

fn default_show_legend() -> bool {
    true
}

/// A chart box and its category dataset.
///
/// `areaChart`, `barChart`, `lineChart`, `stackedAreaChart`, and `stackedBarChart`
/// are plotted. Any other kind keeps the box as a placeholder.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Chart {
    pub element: ReportElement,
    /// Element tag, such as `areaChart`.
    pub kind: String,
    /// `evaluationTime` on `<chart>`, such as `Report`.
    #[serde(default)]
    pub evaluation_time: Option<String>,
    /// Raw `titleExpression` text.
    #[serde(default)]
    pub title: Option<String>,
    /// Raw `subtitleExpression` text.
    #[serde(default)]
    pub subtitle: Option<String>,
    /// JasperReports defaults `isShowLegend` to true.
    #[serde(default = "default_show_legend")]
    pub show_legend: bool,
    /// Array name from `subDataSource("…")` on the chart dataset.
    #[serde(default)]
    pub data_source: Option<String>,
    /// `subDataset` on that dataset run, when the template names one.
    #[serde(default)]
    pub sub_dataset: Option<String>,
    #[serde(default)]
    pub series: Vec<CategorySeries>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Table {
    pub element: ReportElement,
    pub when_no_data: Option<String>,
    pub sub_dataset: Option<String>,
    pub data_source: Option<String>,
    pub columns: Vec<Column>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Column {
    pub width: f32,
    pub sections: Vec<ColumnSection>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SectionKind {
    TableHeader,
    ColumnHeader,
    DetailCell,
    ColumnFooter,
    TableFooter,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnSection {
    pub kind: SectionKind,
    pub style: Option<String>,
    pub height: f32,
    pub elements: Vec<Element>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Element {
    StaticText(StaticText),
    TextField(TextField),
    Line(Graphic),
    Rectangle(Graphic),
    Ellipse(Graphic),
    Image(Image),
    Break(PageBreak),
    Chart(Chart),
    Table(Table),
}

impl Element {
    pub fn report_element(&self) -> &ReportElement {
        match self {
            Element::StaticText(text) => &text.element,
            Element::TextField(field) => &field.element,
            Element::Line(graphic) | Element::Rectangle(graphic) | Element::Ellipse(graphic) => {
                &graphic.element
            }
            Element::Image(image) => &image.element,
            Element::Break(page_break) => &page_break.element,
            Element::Chart(chart) => &chart.element,
            Element::Table(table) => &table.element,
        }
    }
}
