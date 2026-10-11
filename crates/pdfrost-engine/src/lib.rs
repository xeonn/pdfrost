//! Report engine for pdfrost. Callers pass JRXML and one JSON document and
//! receive PDF bytes. [`parse`] reads the template. [`render`] fills fields,
//! lays out tables and pages, and draws the PDF. [`write_report`] writes the
//! supported subset back to JRXML.

mod chart;
mod draw;
mod expr;
mod fill;
mod image;
mod model;
mod parse;
mod subreport;
mod write;

use std::path::Path;

use chrono::{DateTime, Utc};
use serde_json::Value;

pub use image::read_image_bytes;
pub use model::*;
pub use parse::parse;
pub use write::write_report;

/// Liberation Sans Regular (SIL OFL). Used when a template omits `fontName`.
///
/// JasperReports defaults PDF text to Helvetica; Liberation Sans is a
/// metric-compatible, embeddable stand-in that we subset into the PDF.
pub const DEFAULT_SANS_REGULAR: &[u8] = include_bytes!("../fonts/LiberationSans-Regular.ttf");

/// Liberation Sans Bold (SIL OFL).
pub const DEFAULT_SANS_BOLD: &[u8] = include_bytes!("../fonts/LiberationSans-Bold.ttf");

/// Liberation Sans Italic (SIL OFL).
pub const DEFAULT_SANS_ITALIC: &[u8] = include_bytes!("../fonts/LiberationSans-Italic.ttf");

/// Liberation Sans Bold Italic (SIL OFL).
pub const DEFAULT_SANS_BOLD_ITALIC: &[u8] =
    include_bytes!("../fonts/LiberationSans-BoldItalic.ttf");

/// One unsupported tag, expression, or missing value, plus where it was found.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Warning {
    /// Tag name or expression text, such as `chart` or `$F{qty}`.
    pub tag: String,
    /// Place in the template, such as a band name and element index.
    pub location: String,
}

/// PDF bytes, warnings, and the boxes that were laid out.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderOutput {
    pub pdf: Vec<u8>,
    pub warnings: Vec<Warning>,
    /// One box per drawn element. `y` is the distance from the top of the page.
    pub placements: Vec<Placement>,
}

/// A laid-out element, in PDF points, with the origin at the top-left of the page.
#[derive(Debug, Clone, PartialEq)]
pub struct Placement {
    pub kind: PlacementKind,
    /// 1-based page index.
    pub page: u32,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub text: Option<String>,
    /// Stroke width drawn for this box, when it has a pen.
    pub stroke_width: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlacementKind {
    Text,
    Line,
    Rectangle,
    Ellipse,
    /// An image box. `text` is the image expression.
    Image,
    /// A chart box. `text` is the element tag when the series is plotted, or `Chart` for a placeholder.
    Chart,
}

/// Failure that produces no PDF. Warnings alone are not errors.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    #[error("unreadable XML: {0}")]
    Xml(String),
    #[error("root element is not jasperReport")]
    NotAReport,
    #[error("page size must be positive")]
    PageSize,
    #[error("embedded font could not be read")]
    Font,
    #[error("PDF could not be written")]
    Pdf,
}

/// Render `template_xml` with one JSON `document`.
///
/// `rendered_at` is the instant printed for `new java.util.Date()`.
/// Static labels, lines, rectangles, images, `$F` values, and tables are drawn.
/// `new java.util.Date()` prints `rendered_at` as `YYYY-MM-DD HH:MM:SS` in UTC.
/// Image paths are resolved from the process directory. [`render_with_assets`]
/// also looks beside the template.
pub fn render(
    template_xml: &str,
    document: &Value,
    rendered_at: DateTime<Utc>,
) -> Result<RenderOutput, Error> {
    render_with_assets(template_xml, document, rendered_at, None)
}

/// Same as [`render`], and relative image paths are tried in `asset_dir` first.
pub fn render_with_assets(
    template_xml: &str,
    document: &Value,
    rendered_at: DateTime<Utc>,
    asset_dir: Option<&Path>,
) -> Result<RenderOutput, Error> {
    let report = parse(template_xml)?;
    draw::draw(&report, document, rendered_at, asset_dir)
}

#[cfg(test)]
mod krilla_smoke;
