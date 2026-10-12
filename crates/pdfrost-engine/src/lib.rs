//! Report engine for pdfrost. Callers pass JRXML and one JSON document and
//! receive PDF bytes. An object is one record. An array is one record per
//! element. [`parse`] reads the template. [`render`] fills fields, lays out
//! tables and pages, and draws the PDF. [`render_read`] reads a root array one
//! element at a time. [`write_report`] writes the supported subset back to JRXML.

mod chart;
mod draw;
mod expr;
mod fill;
mod image;
mod model;
mod parse;
mod subreport;
mod write;

use std::io::{BufRead, BufReader, Read};
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
    #[error("the document is not JSON: {0}")]
    Json(String),
}

/// Render `template_xml` with one JSON `document`.
///
/// An object is one record: the title, detail, and summary all see it. An array
/// is one record per element. The detail band repeats, the title sees the first
/// element, and the summary sees the last. The array is already in memory here.
/// [`render_read`] reads a root array one element at a time.
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

/// Same as [`render_read_with_assets`] with image paths resolved from the process directory.
pub fn render_read<R: Read>(
    template_xml: &str,
    reader: R,
    rendered_at: DateTime<Utc>,
) -> Result<RenderOutput, Error> {
    render_read_with_assets(template_xml, reader, rendered_at, None)
}

/// Render from a JSON reader.
///
/// A root array is read one element at a time, so a long list of documents does
/// not have to reside as one tree. Any other JSON value is read whole and filled
/// as one record. Relative image paths are tried in `asset_dir` first.
pub fn render_read_with_assets<R: Read>(
    template_xml: &str,
    reader: R,
    rendered_at: DateTime<Utc>,
    asset_dir: Option<&Path>,
) -> Result<RenderOutput, Error> {
    let report = parse(template_xml)?;
    let mut reader = BufReader::new(reader);
    if starts_with_array(&mut reader)? {
        draw::draw_array(&report, reader, rendered_at, asset_dir)
    } else {
        let document =
            serde_json::from_reader(reader).map_err(|err| Error::Json(err.to_string()))?;
        draw::draw(&report, &document, rendered_at, asset_dir)
    }
}

/// True when the next non-space byte is `[`. That byte stays in the buffer.
fn starts_with_array<R: Read>(reader: &mut BufReader<R>) -> Result<bool, Error> {
    let buffered = reader
        .fill_buf()
        .map_err(|err| Error::Json(err.to_string()))?;
    if buffered.starts_with(&[0xEF, 0xBB, 0xBF]) {
        reader.consume(3);
    }
    loop {
        let buffered = reader
            .fill_buf()
            .map_err(|err| Error::Json(err.to_string()))?;
        if buffered.is_empty() {
            return Err(Error::Json("the document is empty".to_string()));
        }
        match buffered[0] {
            b' ' | b'\n' | b'\r' | b'\t' => reader.consume(1),
            b'[' => return Ok(true),
            _ => return Ok(false),
        }
    }
}

#[cfg(test)]
mod krilla_smoke;
