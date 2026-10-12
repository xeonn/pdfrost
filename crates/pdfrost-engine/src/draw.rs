//! Place and paint bands, text, rules, rectangles, images, and tables.
//!
//! A table grows with its rows. Rows that do not fit move to the next page, and that
//! page repeats the table's column header. `$F` inside a table reads the current row.

use std::fmt;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use chrono::{DateTime, Utc};
use krilla::color::rgb;
use krilla::geom::{PathBuilder, Point, Rect, Transform};
use krilla::num::NormalizedF32;
use krilla::page::PageSettings;
use krilla::paint::{Fill, FillRule, LineCap, LineJoin, Stroke};
use krilla::text::{Font, TextDirection};
use krilla::Document;
use serde::de::{Deserializer, SeqAccess, Visitor};
use serde_json::Value;
use skrifa::instance::{LocationRef, Size};
use skrifa::raw::TableProvider;
use skrifa::{FontRef, MetadataProvider};

use crate::fill;
use crate::model::*;
use crate::{
    Error, Placement, PlacementKind, RenderOutput, Warning, DEFAULT_SANS_BOLD,
    DEFAULT_SANS_BOLD_ITALIC, DEFAULT_SANS_ITALIC, DEFAULT_SANS_REGULAR,
};

struct Drawn {
    placement: Placement,
    element: ReportElement,
    body: Body,
}

enum Body {
    Text(String, TextStyle),
    Line,
    Rectangle,
    Ellipse,
    Image(crate::image::Prepared),
    Chart(Option<pdfrost_charts::CategoryChart>),
}

pub(crate) fn draw(
    report: &Report,
    document: &Value,
    rendered_at: DateTime<Utc>,
    asset_dir: Option<&Path>,
) -> Result<RenderOutput, Error> {
    let mut warnings = report.warnings.clone();
    let mut layout = Layout::begin(report, document, rendered_at, asset_dir, &mut warnings)?;
    layout.fill_document(document);
    let (pdf, placements) = layout.finish_output()?;
    Ok(RenderOutput {
        pdf,
        warnings,
        placements,
    })
}

/// Fill a root JSON array, keeping one element at a time.
pub(crate) fn draw_array<R: Read>(
    report: &Report,
    reader: R,
    rendered_at: DateTime<Utc>,
    asset_dir: Option<&Path>,
) -> Result<RenderOutput, Error> {
    let mut warnings = report.warnings.clone();
    let mut layout = Layout::begin(
        report,
        empty_record(),
        rendered_at,
        asset_dir,
        &mut warnings,
    )?;
    let mut deserializer = serde_json::Deserializer::from_reader(reader);
    deserializer
        .deserialize_seq(RecordFill {
            layout: &mut layout,
        })
        .map_err(|err| Error::Json(err.to_string()))?;
    let (pdf, placements) = layout.finish_output()?;
    Ok(RenderOutput {
        pdf,
        warnings,
        placements,
    })
}

/// The record lives with the caller, not inside [`Layout`].
fn borrow_record<'b>(record: *const Value) -> &'b Value {
    // SAFETY: `set_record` stored this address, and that value stays put until
    // the next `set_record` or the end of this fill.
    unsafe { &*record }
}

fn empty_record() -> &'static Value {
    static EMPTY: OnceLock<Value> = OnceLock::new();
    EMPTY.get_or_init(|| Value::Null)
}

fn paint_page(state: &mut PaintState, page: &[Drawn]) {
    let mut pdf_page = state.document.start_page_with(state.settings.clone());
    let mut surface = pdf_page.surface();
    for item in page {
        paint(
            &mut surface,
            item,
            &state.regular,
            &state.bold,
            &state.italic,
            &state.bold_italic,
        );
    }
    surface.finish();
    pdf_page.finish();
    for item in page {
        state.placements.push(item.placement.clone());
    }
}

struct RecordFill<'a, 'l> {
    layout: &'l mut Layout<'a>,
}

impl<'de> Visitor<'de> for RecordFill<'_, '_> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a JSON array of records")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<(), A::Error> {
        let Some(mut current) = seq.next_element::<Value>()? else {
            self.layout.fill_records(&[]);
            return Ok(());
        };
        self.layout.set_record(&current);
        self.layout.open_page();
        self.layout.place_kind(BandKind::Title);
        self.layout.place_kind(BandKind::PageHeader);
        self.layout.place_kind(BandKind::ColumnHeader);
        self.layout.place_kind(BandKind::Detail);
        while let Some(next) = seq.next_element::<Value>()? {
            current = next;
            self.layout.set_record(&current);
            self.layout.place_kind(BandKind::Detail);
        }
        self.layout.place_kind(BandKind::Summary);
        self.layout.finish_page();
        Ok(())
    }
}

fn load_font(bytes: &'static [u8]) -> Result<Font, Error> {
    Font::new(bytes.into(), 0).ok_or(Error::Font)
}

struct PaintState {
    document: Document,
    settings: PageSettings,
    placements: Vec<Placement>,
    regular: Font,
    bold: Font,
    italic: Font,
    bold_italic: Font,
}

struct Layout<'a> {
    report: &'a Report,
    /// Points at the record owned by the caller. It is not stored in this struct.
    record: *const Value,
    rendered_at: DateTime<Utc>,
    asset_dir: Option<&'a Path>,
    warnings: &'a mut Vec<Warning>,
    /// The page being laid out. Finished pages are painted and dropped.
    pages: Vec<Vec<Drawn>>,
    page: u32,
    y: f32,
    /// When false, bands stack in one column. A subreport uses this so its
    /// pages are not pinned to their own paper.
    allow_break: bool,
    /// Canonical paths of subreports already being filled.
    stack: Vec<PathBuf>,
    /// Present for the master report. A subreport leaves this empty and keeps
    /// its drawn items for the caller to splice in.
    paint: Option<PaintState>,
}

impl<'a> Layout<'a> {
    fn begin(
        report: &'a Report,
        document: &'a Value,
        rendered_at: DateTime<Utc>,
        asset_dir: Option<&'a Path>,
        warnings: &'a mut Vec<Warning>,
    ) -> Result<Self, Error> {
        if report.page_width <= 0.0 || report.page_height <= 0.0 {
            return Err(Error::PageSize);
        }
        let settings =
            PageSettings::from_wh(report.page_width, report.page_height).ok_or(Error::PageSize)?;
        Ok(Self {
            report,
            record: document,
            rendered_at,
            asset_dir,
            warnings,
            pages: Vec::new(),
            page: 0,
            y: 0.0,
            allow_break: true,
            stack: Vec::new(),
            paint: Some(PaintState {
                document: Document::new(),
                settings,
                placements: Vec::new(),
                regular: load_font(DEFAULT_SANS_REGULAR)?,
                bold: load_font(DEFAULT_SANS_BOLD)?,
                italic: load_font(DEFAULT_SANS_ITALIC)?,
                bold_italic: load_font(DEFAULT_SANS_BOLD_ITALIC)?,
            }),
        })
    }

    fn set_record(&mut self, record: &Value) {
        self.record = record;
    }

    fn record_ptr(&self) -> *const Value {
        self.record
    }

    fn fill_document(&mut self, document: &'a Value) {
        match document.as_array() {
            Some(items) => self.fill_records(items),
            None => {
                self.set_record(document);
                self.fill_one_record();
            }
        }
    }

    fn fill_records(&mut self, items: &'a [Value]) {
        let Some(first) = items.first() else {
            let empty = Value::Object(Default::default());
            self.set_record(&empty);
            self.open_page();
            self.place_kind(BandKind::Title);
            self.place_kind(BandKind::PageHeader);
            self.place_kind(BandKind::ColumnHeader);
            self.place_kind(BandKind::Summary);
            self.finish_page();
            return;
        };
        self.set_record(first);
        self.open_page();
        self.place_kind(BandKind::Title);
        self.place_kind(BandKind::PageHeader);
        self.place_kind(BandKind::ColumnHeader);
        for item in items {
            self.set_record(item);
            self.place_kind(BandKind::Detail);
        }
        self.place_kind(BandKind::Summary);
        self.finish_page();
    }

    fn fill_one_record(&mut self) {
        self.open_page();
        self.place_kind(BandKind::Title);
        self.place_kind(BandKind::PageHeader);
        self.place_kind(BandKind::ColumnHeader);
        self.place_kind(BandKind::Detail);
        self.place_kind(BandKind::Summary);
        self.finish_page();
    }

    fn finish_output(mut self) -> Result<(Vec<u8>, Vec<Placement>), Error> {
        self.flush_painted_page();
        let paint = self.paint.take().ok_or(Error::Pdf)?;
        let pdf = paint.document.finish().map_err(|_| Error::Pdf)?;
        Ok((pdf, paint.placements))
    }

    /// Paint the page just closed and drop its layout boxes.
    fn flush_painted_page(&mut self) {
        if self.paint.is_none() {
            return;
        }
        let Some(page) = self.pages.pop() else {
            return;
        };
        if let Some(paint) = self.paint.as_mut() {
            paint_page(paint, &page);
        }
    }

    fn limit(&self) -> f32 {
        self.report.page_height - self.report.bottom_margin - footer_height(self.report)
    }

    fn open_page(&mut self) {
        if self.page > 0 {
            self.finish_page();
            self.flush_painted_page();
        }
        self.page += 1;
        self.pages.push(Vec::new());
        self.paint_background();
        self.y = self.report.top_margin;
        if self.page > 1 {
            self.place_fixed(BandKind::PageHeader);
            self.place_fixed(BandKind::ColumnHeader);
        }
    }

    fn finish_page(&mut self) {
        let fields = self.report.fields.clone();
        let record = self.record_ptr();
        let document = borrow_record(record);
        let left = self.report.left_margin;
        let mut top = self.report.page_height - self.report.bottom_margin;
        let page_footers: Vec<Band> = bands(self.report, BandKind::PageFooter).cloned().collect();
        for band in page_footers.iter().rev() {
            top -= band.height;
            self.place_elements(band, left, top, &fields, &document);
        }
        let column_footers: Vec<Band> = bands(self.report, BandKind::ColumnFooter)
            .cloned()
            .collect();
        for band in column_footers.iter().rev() {
            top -= band.height;
            self.place_elements(band, left, top, &fields, &document);
        }
    }

    fn ensure(&mut self, height: f32) {
        if !self.allow_break {
            return;
        }
        if self.page == 0 {
            self.open_page();
        }
        if self.y + height > self.limit() + 0.01 {
            self.open_page();
        }
    }

    fn paint_background(&mut self) {
        let fields = self.report.fields.clone();
        let record = self.record_ptr();
        let document = borrow_record(record);
        let left = self.report.left_margin;
        let top = self.report.top_margin;
        let backgrounds: Vec<Band> = bands(self.report, BandKind::Background).cloned().collect();
        for band in &backgrounds {
            self.place_elements(band, left, top, &fields, &document);
        }
    }

    fn place_kind(&mut self, kind: BandKind) {
        let found: Vec<Band> = bands(self.report, kind).cloned().collect();
        for band in &found {
            self.place_flow(band);
        }
    }

    fn place_fixed(&mut self, kind: BandKind) {
        let fields = self.report.fields.clone();
        let record = self.record_ptr();
        let document = borrow_record(record);
        let left = self.report.left_margin;
        let found: Vec<Band> = bands(self.report, kind).cloned().collect();
        for band in &found {
            let y = self.y;
            self.place_elements(band, left, y, &fields, &document);
            self.y += band.height;
        }
    }

    fn place_flow(&mut self, band: &Band) {
        let record = self.record_ptr();
        let document = borrow_record(record);
        self.place_flow_data(band, document);
    }

    fn place_flow_data(&mut self, band: &Band, document: &Value) {
        self.ensure(band.height);
        let band_top = self.y;
        let start_page = self.page;
        let fields = self.report.fields.clone();
        let left = self.report.left_margin;
        let mut origin_y = band_top;
        let mut stretches: Vec<(f32, f32)> = Vec::new();
        let mut band_bottom = band.height;
        for element in &band.elements {
            if matches!(element, Element::Table(_)) {
                continue;
            }
            if let Element::Break(_) = element {
                if self.allow_break
                    && matches!(
                        band.kind,
                        BandKind::Title | BandKind::Detail | BandKind::Summary
                    )
                {
                    self.open_page();
                    origin_y = self.y;
                    stretches.clear();
                    band_bottom = band.height;
                }
                continue;
            }
            if let Element::Subreport(subreport) = element {
                let y_shift = float_shift(&subreport.element, &stretches);
                let block = self.embed_subreport(subreport, &fields, document);
                let content = block.height;
                let used = content.max(subreport.element.height);
                let top = origin_y + subreport.element.y + y_shift;
                let page_before = self.page;
                self.blit_block(block, left + subreport.element.x, top, true);
                if self.page != page_before {
                    origin_y = self.y;
                    stretches.clear();
                    band_bottom = 0.0;
                    continue;
                }
                band_bottom = band_bottom.max(subreport.element.y + y_shift + used);
                let extra = (content - subreport.element.height).max(0.0);
                if extra > 0.0 {
                    stretches.push((subreport.element.y + subreport.element.height, extra));
                }
                continue;
            }
            let frame = element.report_element();
            let y_shift = float_shift(frame, &stretches);
            let extra = self.place_one(element, left, origin_y + y_shift, &fields, &document, true);
            let bottom = frame.y + y_shift + frame.height + extra;
            band_bottom = band_bottom.max(bottom);
            if extra > 0.0 {
                stretches.push((frame.y + frame.height, extra));
            }
        }
        for element in &band.elements {
            let Element::Table(table) = element else {
                continue;
            };
            if self.page != start_page {
                break;
            }
            let y_shift = float_shift(&table.element, &stretches);
            self.y = self.y.max(band_top + table.element.y + y_shift);
            self.place_table(table, document);
            band_bottom = band_bottom.max(self.y - band_top);
        }
        if self.page == start_page {
            self.y = self.y.max(band_top + band_bottom);
        } else {
            self.y = self.y.max(origin_y + band_bottom);
        }
    }

    fn place_table(&mut self, table: &Table, data: &Value) {
        let abs_x = self.report.left_margin + table.element.x;
        let fields = dataset_fields(self.report, table.sub_dataset.as_deref()).to_vec();
        let rows = table_rows(data, table.data_source.as_deref());
        self.place_table_section(table, abs_x, SectionKind::TableHeader, None, &fields);
        self.place_table_section(table, abs_x, SectionKind::ColumnHeader, None, &fields);
        let row_height = section_height(table, SectionKind::DetailCell).unwrap_or(0.0);
        for row in &rows {
            if self.allow_break && row_height > 0.0 && self.y + row_height > self.limit() + 0.01 {
                self.open_page();
                self.place_table_section(table, abs_x, SectionKind::ColumnHeader, None, &fields);
            }
            self.place_table_section(table, abs_x, SectionKind::DetailCell, Some(row), &fields);
        }
        self.place_table_section(table, abs_x, SectionKind::ColumnFooter, None, &fields);
        self.place_table_section(table, abs_x, SectionKind::TableFooter, None, &fields);
    }

    fn place_table_section(
        &mut self,
        table: &Table,
        abs_x: f32,
        kind: SectionKind,
        row: Option<&Value>,
        fields: &[Field],
    ) {
        let Some(height) = section_height(table, kind) else {
            return;
        };
        if height > 0.0 {
            self.ensure(height);
        }
        let y = self.y;
        let record = self.record_ptr();
        let fallback = borrow_record(record);
        let data = row.unwrap_or(fallback);
        let mut x = abs_x;
        for column in &table.columns {
            if let Some(section) = column.sections.iter().find(|section| section.kind == kind) {
                self.paint_cell(x, y, column.width, section, data, fields);
            }
            x += column.width;
        }
        self.y += height;
    }

    fn paint_cell(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        section: &ColumnSection,
        data: &Value,
        fields: &[Field],
    ) {
        let frame = cell_frame(self.report, section, width);
        let stroke_width = pen_width(&frame);
        self.push(Drawn {
            placement: Placement {
                kind: PlacementKind::Rectangle,
                page: self.page,
                x,
                y,
                width,
                height: section.height,
                text: None,
                stroke_width,
            },
            element: frame,
            body: Body::Rectangle,
        });
        for element in &section.elements {
            self.place_one(element, x, y, fields, data, false);
        }
    }

    fn place_elements(
        &mut self,
        band: &Band,
        origin_x: f32,
        origin_y: f32,
        fields: &[Field],
        data: &Value,
    ) {
        for element in &band.elements {
            if let Element::Subreport(subreport) = element {
                let block = self.embed_subreport(subreport, fields, data);
                self.blit_block(
                    block,
                    origin_x + subreport.element.x,
                    origin_y + subreport.element.y,
                    false,
                );
                continue;
            }
            if matches!(element, Element::Table(_)) {
                continue;
            }
            self.place_one(element, origin_x, origin_y, fields, data, false);
        }
    }

    fn place_one(
        &mut self,
        element: &Element,
        origin_x: f32,
        origin_y: f32,
        fields: &[Field],
        data: &Value,
        stretch: bool,
    ) -> f32 {
        match element {
            Element::StaticText(text) => self.place_text(
                origin_x,
                origin_y,
                &text.element,
                &text.text,
                &text.text_style,
                stretch && text.stretch,
            ),
            Element::TextField(field) => {
                let filled = fill::eval(
                    &field.expression,
                    fields,
                    data,
                    "textField",
                    self.page,
                    self.rendered_at,
                );
                if let Some(warning) = filled.warning {
                    self.warnings.push(warning);
                }
                self.place_text(
                    origin_x,
                    origin_y,
                    &field.element,
                    &filled.text,
                    &field.text_style,
                    stretch && field.stretch,
                )
            }
            Element::Line(graphic) => {
                self.push_graphic(
                    origin_x + graphic.element.x,
                    origin_y + graphic.element.y,
                    &graphic.element,
                    PlacementKind::Line,
                    None,
                    Body::Line,
                );
                0.0
            }
            Element::Rectangle(graphic) => {
                self.push_graphic(
                    origin_x + graphic.element.x,
                    origin_y + graphic.element.y,
                    &graphic.element,
                    PlacementKind::Rectangle,
                    None,
                    Body::Rectangle,
                );
                0.0
            }
            Element::Ellipse(graphic) => {
                self.push_graphic(
                    origin_x + graphic.element.x,
                    origin_y + graphic.element.y,
                    &graphic.element,
                    PlacementKind::Ellipse,
                    None,
                    Body::Ellipse,
                );
                0.0
            }
            Element::Image(image) => {
                let prepared =
                    crate::image::prepare(image, fields, data, self.asset_dir, self.warnings);
                let extra = prepared.extra_height();
                let mut grown = image.element.clone();
                grown.height += extra;
                self.push_graphic(
                    origin_x + image.element.x,
                    origin_y + image.element.y,
                    &grown,
                    PlacementKind::Image,
                    Some(image.expression.clone()),
                    Body::Image(prepared),
                );
                extra
            }
            Element::Break(_) => 0.0,
            Element::Subreport(_) => 0.0,
            Element::Chart(chart) => {
                let plot = crate::chart::prepare(
                    chart,
                    fields,
                    &self.report.subdatasets,
                    data,
                    self.warnings,
                );
                let label = if plot.is_some() {
                    chart.kind.clone()
                } else {
                    "Chart".to_string()
                };
                self.push_graphic(
                    origin_x + chart.element.x,
                    origin_y + chart.element.y,
                    &chart.element,
                    PlacementKind::Chart,
                    Some(label),
                    Body::Chart(plot),
                );
                0.0
            }
            Element::Table(_) => 0.0,
        }
    }

    fn place_text(
        &mut self,
        origin_x: f32,
        origin_y: f32,
        element: &ReportElement,
        text: &str,
        style: &TextStyle,
        stretch: bool,
    ) -> f32 {
        let height = if stretch {
            stretched_height(element, text, style)
        } else {
            element.height
        };
        let mut grown = element.clone();
        grown.height = height;
        self.push_graphic(
            origin_x + element.x,
            origin_y + element.y,
            &grown,
            PlacementKind::Text,
            Some(text.to_string()),
            Body::Text(text.to_string(), style.clone()),
        );
        (height - element.height).max(0.0)
    }

    fn push_graphic(
        &mut self,
        x: f32,
        y: f32,
        element: &ReportElement,
        kind: PlacementKind,
        text: Option<String>,
        body: Body,
    ) {
        let stroke_width = match kind {
            PlacementKind::Text => pen_width(element),
            PlacementKind::Line
            | PlacementKind::Rectangle
            | PlacementKind::Ellipse
            | PlacementKind::Image => positive_pen_width(element),
            PlacementKind::Chart => Some(1.0),
        };
        self.push(Drawn {
            placement: Placement {
                kind,
                page: self.page,
                x,
                y,
                width: element.width,
                height: element.height,
                text,
                stroke_width,
            },
            element: element.clone(),
            body,
        });
    }

    fn push(&mut self, drawn: Drawn) {
        if let Some(page) = self.pages.last_mut() {
            page.push(drawn);
        }
    }

    fn blit_block(&mut self, block: Block, x: f32, mut y: f32, flow: bool) {
        if !flow {
            for mut item in block.items {
                item.placement.x += x;
                item.placement.y += y;
                item.placement.page = self.page;
                self.push(item);
            }
            return;
        }
        let mut items = block.items;
        let mut breaks = block.breaks;
        let mut height = block.height;
        for _ in 0..32 {
            if items.is_empty() {
                self.y = self.y.max(y);
                return;
            }
            let room = self.limit() - y;
            if height <= room + 0.01 {
                for mut item in items {
                    item.placement.x += x;
                    item.placement.y += y;
                    item.placement.page = self.page;
                    self.push(item);
                }
                self.y = self.y.max(y + height);
                return;
            }
            let at_top = y <= self.report.top_margin + 0.5;
            let cut = next_cut(&items, &breaks, room, at_top);
            if cut <= 0.5 {
                self.open_page();
                y = self.y;
                continue;
            }
            let mut rest = Vec::new();
            for mut item in items.drain(..) {
                let top = item.placement.y;
                let bottom = top + item.placement.height;
                if bottom <= cut + 0.01 {
                    item.placement.x += x;
                    item.placement.y += y;
                    item.placement.page = self.page;
                    self.push(item);
                } else if top + 0.01 >= cut {
                    item.placement.y -= cut;
                    rest.push(item);
                } else {
                    item.placement.x += x;
                    item.placement.y += y;
                    item.placement.page = self.page;
                    self.push(item);
                }
            }
            items = rest;
            breaks = breaks
                .into_iter()
                .filter_map(|boundary| {
                    if boundary > cut + 0.01 {
                        Some(boundary - cut)
                    } else {
                        None
                    }
                })
                .collect();
            height = (height - cut).max(0.0);
            self.open_page();
            y = self.y;
        }
    }

    fn embed_subreport(&mut self, sub: &Subreport, fields: &[Field], data: &Value) -> Block {
        if self.stack.len() >= 8 {
            self.warnings.push(Warning {
                tag: "subreport".to_string(),
                location: "nested".to_string(),
            });
            return Block::empty();
        }
        let Some(source) =
            crate::subreport::expression_path(&sub.expression, fields, data, self.warnings)
        else {
            return Block::empty();
        };
        let Some(path) = crate::subreport::locate(&source, self.asset_dir, self.warnings) else {
            return Block::empty();
        };
        let canonical = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
        if self.stack.iter().any(|item| item == &canonical) {
            self.warnings.push(Warning {
                tag: "subreport cycle".to_string(),
                location: source,
            });
            return Block::empty();
        }
        let xml = match std::fs::read_to_string(&path) {
            Ok(xml) => xml,
            Err(_) => {
                self.warnings.push(Warning {
                    tag: format!("subreport {source}"),
                    location: "subreport".to_string(),
                });
                return Block::empty();
            }
        };
        let mut child = match crate::parse(&xml) {
            Ok(report) => report,
            Err(err) => {
                self.warnings.push(Warning {
                    tag: format!("subreport {err}"),
                    location: "subreport".to_string(),
                });
                return Block::empty();
            }
        };
        for warning in child.warnings.drain(..) {
            self.warnings.push(Warning {
                tag: warning.tag,
                location: format!("subreport {}", warning.location),
            });
        }
        let records = match crate::subreport::records(data, sub.data_source.as_deref()) {
            Ok(records) => records,
            Err(()) => {
                let path = sub.data_source.as_deref().unwrap_or("");
                self.warnings.push(Warning {
                    tag: format!("subreport {path}"),
                    location: "dataSourceExpression".to_string(),
                });
                return Block::empty();
            }
        };
        if records.is_empty() {
            return Block::empty();
        }
        child.left_margin = 0.0;
        child.right_margin = 0.0;
        child.top_margin = 0.0;
        child.bottom_margin = 0.0;
        child.page_width = sub.element.width.max(1.0);
        child.page_height = 1_000_000.0;
        let mut stack = self.stack.clone();
        stack.push(canonical);
        let child_dir = path.parent().map(|dir| dir.to_path_buf());
        stack_report(
            &child,
            records,
            self.rendered_at,
            child_dir.as_deref(),
            self.warnings,
            stack,
            self.page,
        )
    }
}

struct Block {
    items: Vec<Drawn>,
    height: f32,
    /// Y positions, in the block, after each detail record. A page break prefers these.
    breaks: Vec<f32>,
}

impl Block {
    fn empty() -> Self {
        Self {
            items: Vec::new(),
            height: 0.0,
            breaks: Vec::new(),
        }
    }
}

/// Largest y within `room` that ends a detail record, else a row that fits.
fn next_cut(items: &[Drawn], breaks: &[f32], room: f32, at_top: bool) -> f32 {
    if let Some(cut) = breaks
        .iter()
        .copied()
        .filter(|boundary| *boundary > 0.5 && *boundary <= room + 0.01)
        .reduce(f32::max)
    {
        return cut;
    }
    if let Some(cut) = item_cut(items, room) {
        return cut;
    }
    if at_top {
        return items
            .iter()
            .map(|item| item.placement.y + item.placement.height)
            .filter(|bottom| *bottom > 0.5)
            .reduce(f32::min)
            .unwrap_or(0.0);
    }
    0.0
}

fn item_cut(items: &[Drawn], room: f32) -> Option<f32> {
    let mut best = None;
    for item in items {
        let bottom = item.placement.y + item.placement.height;
        if bottom <= 0.5 || bottom > room + 0.01 {
            continue;
        }
        let straddles = items.iter().any(|other| {
            let top = other.placement.y;
            let end = top + other.placement.height;
            top < bottom - 0.01 && end > bottom + 0.01
        });
        if !straddles {
            best = Some(best.map_or(bottom, |current: f32| current.max(bottom)));
        }
    }
    best
}

fn stack_report(
    report: &Report,
    records: crate::subreport::Records<'_>,
    rendered_at: DateTime<Utc>,
    asset_dir: Option<&Path>,
    warnings: &mut Vec<Warning>,
    stack: Vec<PathBuf>,
    page: u32,
) -> Block {
    let Some(first) = records.get(0) else {
        return Block::empty();
    };
    let mut layout = Layout {
        report,
        record: first,
        rendered_at,
        asset_dir,
        warnings,
        pages: vec![Vec::new()],
        page: page.max(1),
        y: 0.0,
        allow_break: false,
        stack,
        paint: None,
    };
    for kind in [
        BandKind::Background,
        BandKind::Title,
        BandKind::PageHeader,
        BandKind::ColumnHeader,
    ] {
        layout.place_kind(kind);
    }
    let details: Vec<Band> = bands(report, BandKind::Detail).cloned().collect();
    let mut breaks = Vec::new();
    for index in 0..records.len() {
        let Some(record) = records.get(index) else {
            continue;
        };
        layout.set_record(record);
        for band in &details {
            layout.place_flow_data(band, record);
        }
        breaks.push(layout.y);
    }
    for kind in [
        BandKind::ColumnFooter,
        BandKind::PageFooter,
        BandKind::Summary,
    ] {
        layout.place_kind(kind);
    }
    Block {
        height: layout.y,
        items: layout.pages.into_iter().next().unwrap_or_default(),
        breaks,
    }
}

fn bands(report: &Report, kind: BandKind) -> impl Iterator<Item = &Band> {
    report.bands.iter().filter(move |band| band.kind == kind)
}

fn footer_height(report: &Report) -> f32 {
    bands(report, BandKind::PageFooter)
        .chain(bands(report, BandKind::ColumnFooter))
        .map(|band| band.height)
        .sum()
}

fn section_height(table: &Table, kind: SectionKind) -> Option<f32> {
    let mut height = None;
    for column in &table.columns {
        if let Some(section) = column.sections.iter().find(|section| section.kind == kind) {
            height = Some(height.unwrap_or(0.0_f32).max(section.height));
        }
    }
    height
}

fn table_rows<'a>(document: &'a Value, source: Option<&str>) -> Vec<&'a Value> {
    let Some(name) = source else {
        return Vec::new();
    };
    match document.get(name) {
        Some(Value::Array(items)) => items.iter().collect(),
        _ => Vec::new(),
    }
}

fn dataset_fields<'a>(report: &'a Report, name: Option<&str>) -> &'a [Field] {
    name.and_then(|name| {
        report
            .subdatasets
            .iter()
            .find(|dataset| dataset.name == name)
    })
    .map(|dataset| dataset.fields.as_slice())
    .unwrap_or(&[])
}

fn cell_frame(report: &Report, section: &ColumnSection, width: f32) -> ReportElement {
    let mut element = ReportElement {
        width,
        height: section.height,
        ..ReportElement::default()
    };
    if let Some(style) = section
        .style
        .as_deref()
        .and_then(|name| report.styles.iter().find(|style| style.name == name))
    {
        element.mode = style.mode;
        element.backcolor = style.backcolor;
        element.borders = style.borders.clone();
    }
    element
}

fn pen_width(element: &ReportElement) -> Option<f32> {
    let borders = element.borders.as_ref()?;
    borders
        .pen
        .as_ref()
        .or(borders.top.as_ref())
        .or(borders.left.as_ref())
        .or(borders.bottom.as_ref())
        .or(borders.right.as_ref())
        .and_then(|pen| pen.line_width)
}

/// JasperReports resolves a missing line width to 0, and a 0-width pen is not stroked.
fn positive_pen_width(element: &ReportElement) -> Option<f32> {
    pen_width(element).filter(|width| *width > 0.0)
}

fn paint(
    surface: &mut krilla::surface::Surface<'_>,
    item: &Drawn,
    regular: &Font,
    bold: &Font,
    italic: &Font,
    bold_italic: &Font,
) {
    let x = item.placement.x;
    let y = item.placement.y;
    let width = item.placement.width;
    let height = item.placement.height;
    if width <= 0.0 || height <= 0.0 {
        return;
    }

    if item.element.mode == Some(Mode::Opaque) && !matches!(item.body, Body::Ellipse) {
        let color = item.element.backcolor.unwrap_or(Color {
            r: 255,
            g: 255,
            b: 255,
        });
        fill_rect(surface, x, y, width, height, color, 255);
    }

    match &item.body {
        Body::Line => {
            let pen = item
                .element
                .borders
                .as_ref()
                .and_then(|borders| borders.pen.as_ref());
            stroke_line(surface, x, y, x + width, y + height, pen);
        }
        Body::Rectangle => {
            if let Some(borders) = &item.element.borders {
                stroke_box(surface, x, y, width, height, borders);
            } else {
                stroke_rect(surface, x, y, width, height, None);
            }
        }
        Body::Ellipse => {
            paint_ellipse(surface, x, y, width, height, &item.element);
        }
        Body::Image(prepared) => {
            paint_image(surface, x, y, width, height, prepared);
            if let Some(borders) = &item.element.borders {
                stroke_box(surface, x, y, width, height, borders);
            }
        }
        Body::Chart(plot) => {
            if let Some(plot) = plot {
                let fonts = pdfrost_charts::SkrifaFace {
                    regular: DEFAULT_SANS_REGULAR,
                    bold: DEFAULT_SANS_BOLD,
                };
                let mut record = pdfrost_charts::Record::default();
                pdfrost_charts::draw(plot, width, height, &fonts, &mut record);
                paint_chart(surface, x, y, &record, regular, bold);
            } else {
                fill_rect(
                    surface,
                    x,
                    y,
                    width,
                    height,
                    Color {
                        r: 243,
                        g: 244,
                        b: 246,
                    },
                    255,
                );
                let pen = Pen {
                    line_width: Some(1.0),
                    line_color: Some(Color {
                        r: 156,
                        g: 163,
                        b: 175,
                    }),
                    line_style: Some("Solid".to_string()),
                };
                stroke_rect(surface, x, y, width, height, Some(&pen));
                let label = item.placement.text.clone().unwrap_or_default();
                draw_label(
                    surface,
                    item,
                    &label,
                    &TextStyle {
                        text_alignment: Some(TextAlignment::Center),
                        vertical_alignment: Some(VerticalAlignment::Middle),
                        font_size: Some(12.0),
                        ..TextStyle::default()
                    },
                    &Fonts {
                        regular,
                        bold,
                        italic,
                        bold_italic,
                    },
                );
            }
        }
        Body::Text(text, style) => {
            if let Some(borders) = &item.element.borders {
                stroke_box(surface, x, y, width, height, borders);
            }
            if let Some(path) = rect_path(x, y, width, height) {
                surface.push_clip_path(&path, &FillRule::NonZero);
                draw_label(
                    surface,
                    item,
                    text,
                    style,
                    &Fonts {
                        regular,
                        bold,
                        italic,
                        bold_italic,
                    },
                );
                surface.pop();
            }
        }
    }
}

struct Fonts<'a> {
    regular: &'a Font,
    bold: &'a Font,
    italic: &'a Font,
    bold_italic: &'a Font,
}

fn draw_label(
    surface: &mut krilla::surface::Surface<'_>,
    item: &Drawn,
    text: &str,
    style: &TextStyle,
    fonts: &Fonts<'_>,
) {
    if text.is_empty() {
        return;
    }
    let (x, y, width, height) = content_box(item);
    let (layout_width, layout_height) = match style.rotation {
        Some(Rotation::Left | Rotation::Right) => (height, width),
        Some(Rotation::UpsideDown) | None => (width, height),
    };
    surface.push_transform(&text_transform(style.rotation, x, y, width, height));
    let pad_left = if style.rotation.is_none() {
        item.element
            .borders
            .as_ref()
            .map(|borders| borders.padding_left)
            .unwrap_or(0.0)
    } else {
        0.0
    };
    for run in place_text(text, style, 0.0, 0.0, layout_width, layout_height, pad_left) {
        let (font, bytes) = face_run(&run.style, fonts);
        let vertical = font_vertical(bytes, run.style.size);
        if let Some(back) = run.style.backcolor {
            let text_width = measure(bytes, &run.text, run.style.size);
            fill_rect(
                surface,
                run.x,
                run.y - vertical.ascent,
                text_width,
                vertical.ascent + vertical.descent,
                back,
                255,
            );
        }
        surface.set_stroke(None);
        surface.set_fill(Some(fill(run.style.color)));
        surface.draw_text(
            Point::from_xy(run.x, run.y + run.style.shift),
            font.clone(),
            run.style.size,
            &run.text,
            false,
            TextDirection::Auto,
        );
        let text_width = measure(bytes, &run.text, run.style.size);
        if run.style.underline {
            paint_decoration(
                surface,
                run.x,
                run.y,
                text_width,
                vertical.underline,
                run.style.color,
            );
        }
        if run.style.strike {
            paint_decoration(
                surface,
                run.x,
                run.y,
                text_width,
                vertical.strike,
                run.style.color,
            );
        }
    }
    surface.pop();
}

struct TextRun {
    text: String,
    x: f32,
    /// Baseline, measured from the top of the page.
    y: f32,
    style: RunStyle,
}

#[derive(Clone, Debug)]
struct RunStyle {
    bold: bool,
    italic: bool,
    underline: bool,
    strike: bool,
    size: f32,
    color: Color,
    backcolor: Option<Color>,
    /// Added to the baseline. Positive moves the glyphs down.
    shift: f32,
}

impl RunStyle {
    fn base(style: &TextStyle) -> Self {
        Self {
            bold: style.bold,
            italic: style.italic,
            underline: style.underline,
            strike: style.strike_through,
            size: style.font_size.unwrap_or(10.0).max(0.0),
            color: Color { r: 0, g: 0, b: 0 },
            backcolor: None,
            shift: 0.0,
        }
    }

    fn same_face(&self, other: &Self) -> bool {
        self.bold == other.bold
            && self.italic == other.italic
            && self.underline == other.underline
            && self.strike == other.strike
            && self.size == other.size
            && self.color == other.color
            && self.backcolor == other.backcolor
            && self.shift == other.shift
    }
}

struct Word {
    text: String,
    width: f32,
    style: RunStyle,
}

struct Segment {
    words: Vec<Word>,
    width: f32,
    /// Content-box x before horizontal alignment.
    start: f32,
}

struct WrappedLine {
    segments: Vec<Segment>,
    /// The last line of a paragraph. Justified text leaves this line on the left.
    paragraph_end: bool,
}

/// A `Float` element moves down by the stretch of every element whose original bottom is at or above its `y`.
fn float_shift(element: &ReportElement, stretches: &[(f32, f32)]) -> f32 {
    if element.position == PositionType::FixRelativeToTop {
        return 0.0;
    }
    stretches
        .iter()
        .filter(|(bottom, _)| *bottom <= element.y + 0.01)
        .map(|(_, extra)| *extra)
        .sum()
}

fn face_run<'a>(style: &RunStyle, fonts: &Fonts<'a>) -> (&'a Font, &'static [u8]) {
    match (style.bold, style.italic) {
        (true, true) => (fonts.bold_italic, DEFAULT_SANS_BOLD_ITALIC),
        (true, false) => (fonts.bold, DEFAULT_SANS_BOLD),
        (false, true) => (fonts.italic, DEFAULT_SANS_ITALIC),
        (false, false) => (fonts.regular, DEFAULT_SANS_REGULAR),
    }
}

fn run_bytes(style: &RunStyle) -> &'static [u8] {
    match (style.bold, style.italic) {
        (true, true) => DEFAULT_SANS_BOLD_ITALIC,
        (true, false) => DEFAULT_SANS_BOLD,
        (false, true) => DEFAULT_SANS_ITALIC,
        (false, false) => DEFAULT_SANS_REGULAR,
    }
}

fn font_bytes(style: &TextStyle) -> &'static [u8] {
    match (style.bold, style.italic) {
        (true, true) => DEFAULT_SANS_BOLD_ITALIC,
        (true, false) => DEFAULT_SANS_BOLD,
        (false, true) => DEFAULT_SANS_ITALIC,
        (false, false) => DEFAULT_SANS_REGULAR,
    }
}

fn content_box(item: &Drawn) -> (f32, f32, f32, f32) {
    let pad = item.element.borders.as_ref();
    let top = pad.map(|borders| borders.padding_top).unwrap_or(0.0);
    let left = pad.map(|borders| borders.padding_left).unwrap_or(0.0);
    let bottom = pad.map(|borders| borders.padding_bottom).unwrap_or(0.0);
    let right = pad.map(|borders| borders.padding_right).unwrap_or(0.0);
    (
        item.placement.x + left,
        item.placement.y + top,
        (item.placement.width - left - right).max(0.0),
        (item.placement.height - top - bottom).max(0.0),
    )
}

fn text_transform(
    rotation: Option<Rotation>,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) -> Transform {
    match rotation {
        Some(Rotation::Left) => Transform::from_row(0.0, -1.0, 1.0, 0.0, x, y + height),
        Some(Rotation::Right) => Transform::from_row(0.0, 1.0, -1.0, 0.0, x + width, y),
        Some(Rotation::UpsideDown) => {
            Transform::from_row(-1.0, 0.0, 0.0, -1.0, x + width, y + height)
        }
        None => Transform::from_translate(x, y),
    }
}

struct FontVertical {
    ascent: f32,
    descent: f32,
    leading: f32,
    /// Stroke center below the baseline (positive is down) and thickness.
    underline: (f32, f32),
    strike: (f32, f32),
}

fn font_vertical(bytes: &[u8], size: f32) -> FontVertical {
    let fallback = FontVertical {
        ascent: size,
        descent: size * 0.2119,
        leading: size * 0.0327,
        underline: (size * 0.07, (size / 18.0).max(0.4)),
        strike: (-size * 0.23, (size * 0.05).max(0.4)),
    };
    if size <= 0.0 {
        return fallback;
    }
    let Ok(font) = FontRef::from_index(bytes, 0) else {
        return fallback;
    };
    let metrics = font.metrics(Size::new(size), LocationRef::default());
    let ascent = if metrics.ascent > 0.0 {
        metrics.ascent
    } else {
        fallback.ascent
    };
    let descent = if metrics.descent < 0.0 {
        -metrics.descent
    } else if metrics.descent > 0.0 {
        metrics.descent
    } else {
        fallback.descent
    };
    let decoration = |item: Option<skrifa::metrics::Decoration>, fallback: (f32, f32)| {
        item.map(|item| {
            let thickness = if item.thickness > 0.0 {
                item.thickness
            } else {
                fallback.1
            };
            // The font offset is the top of the decoration in a y-up space.
            (-item.offset + thickness / 2.0, thickness)
        })
        .unwrap_or(fallback)
    };
    FontVertical {
        ascent,
        descent,
        leading: metrics.leading.max(0.0),
        underline: decoration(metrics.underline, fallback.underline),
        strike: decoration(metrics.strikeout, fallback.strike),
    }
}

/// JasperReports `getLineHeight`. The first line of the element always uses single spacing.
fn jasper_line_height(first: bool, style: &TextStyle, vertical: &FontVertical) -> f32 {
    let single = vertical.leading + vertical.ascent;
    if first {
        return single;
    }
    match style.line_spacing {
        Some(LineSpacing::OneAndHalf) => vertical.leading + 1.5 * vertical.ascent,
        Some(LineSpacing::Double) => vertical.leading + 2.0 * vertical.ascent,
        Some(LineSpacing::Proportional) => {
            vertical.leading + style.line_spacing_size.unwrap_or(1.0) * vertical.ascent
        }
        Some(LineSpacing::AtLeast) => single.max(style.line_spacing_size.unwrap_or(single)),
        Some(LineSpacing::Fixed) => style.line_spacing_size.unwrap_or(single),
        Some(LineSpacing::Single) | None => single,
    }
}

fn stretched_height(element: &ReportElement, text: &str, style: &TextStyle) -> f32 {
    if style.rotation.is_some() || text.is_empty() {
        return element.height;
    }
    let pad = element.borders.as_ref();
    let top = pad.map(|borders| borders.padding_top).unwrap_or(0.0);
    let bottom = pad.map(|borders| borders.padding_bottom).unwrap_or(0.0);
    let left = pad.map(|borders| borders.padding_left).unwrap_or(0.0);
    let right = pad.map(|borders| borders.padding_right).unwrap_or(0.0);
    let content_width = (element.width - left - right).max(0.0);
    let lines = layout_lines(text, style, content_width, left);
    let block = text_block(&lines, style, f32::MAX).1;
    (top + bottom + block).max(element.height)
}

/// Lay out text the way JasperReports 6.20.1 measures it: font leading and ascent, then descent.
fn place_text(
    text: &str,
    style: &TextStyle,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    pad_left: f32,
) -> Vec<TextRun> {
    let lines = layout_lines(text, style, width, pad_left);
    let (baselines, block, fitted) = text_block(&lines, style, height);
    if fitted == 0 {
        return Vec::new();
    }
    let offset = match style.vertical_alignment {
        Some(VerticalAlignment::Middle) => (height - block) / 2.0,
        Some(VerticalAlignment::Bottom) => height - block,
        Some(VerticalAlignment::Top) | None => 0.0,
    };
    let right_limit = (width - style.right_indent.unwrap_or(0.0)).max(0.0);
    let mut runs = Vec::new();
    for (line, baseline) in lines.iter().take(fitted).zip(baselines) {
        let line_end = line
            .segments
            .last()
            .map(|segment| segment.start + segment.width)
            .unwrap_or(0.0);
        let justify = matches!(style.text_alignment, Some(TextAlignment::Justified))
            && !line.paragraph_end
            && line.segments.len() == 1
            && line.segments[0].words.len() > 1;
        let extra = if justify {
            0.0
        } else {
            match style.text_alignment {
                Some(TextAlignment::Center) => (right_limit - line_end) / 2.0,
                Some(TextAlignment::Right) => right_limit - line_end,
                Some(TextAlignment::Left | TextAlignment::Justified) | None => 0.0,
            }
        };
        for segment in &line.segments {
            let gaps = segment.words.len().saturating_sub(1);
            if justify && gaps > 0 {
                let spare = (right_limit - (segment.start + segment.width)).max(0.0);
                let mut cursor = x + segment.start + extra;
                for (index, word) in segment.words.iter().enumerate() {
                    runs.push(TextRun {
                        text: word.text.clone(),
                        x: cursor,
                        y: y + offset + baseline,
                        style: word.style.clone(),
                    });
                    cursor += word.width;
                    if index + 1 < segment.words.len() {
                        let space = measure(run_bytes(&word.style), " ", word.style.size);
                        cursor += space + spare / gaps as f32;
                    }
                }
                continue;
            }
            let mut cursor = x + segment.start + extra;
            let mut joined = String::new();
            let mut joined_style: Option<RunStyle> = None;
            let flush = |joined: &mut String,
                         joined_style: &mut Option<RunStyle>,
                         cursor: f32,
                         runs: &mut Vec<TextRun>| {
                if joined.is_empty() {
                    return cursor;
                }
                let run_style = joined_style.take().unwrap();
                let width = measure(run_bytes(&run_style), joined, run_style.size);
                runs.push(TextRun {
                    text: std::mem::take(joined),
                    x: cursor - width,
                    y: y + offset + baseline,
                    style: run_style,
                });
                cursor
            };
            for (index, word) in segment.words.iter().enumerate() {
                let space = if index == 0 {
                    0.0
                } else {
                    measure(run_bytes(&word.style), " ", word.style.size)
                };
                let continue_run = joined_style
                    .as_ref()
                    .is_some_and(|current| current.same_face(&word.style));
                if !continue_run {
                    cursor = flush(&mut joined, &mut joined_style, cursor, &mut runs);
                    cursor += space;
                    joined_style = Some(word.style.clone());
                } else if index > 0 {
                    joined.push(' ');
                }
                joined.push_str(&word.text);
                cursor += space + word.width;
                if !continue_run {
                    cursor -= space;
                }
            }
            flush(&mut joined, &mut joined_style, cursor, &mut runs);
        }
    }
    runs
}

/// Baselines of the lines that fit, the measured text height, and how many lines fit.
/// A baseline is the distance from the top of the content box before vertical alignment.
fn text_block(lines: &[WrappedLine], style: &TextStyle, height: f32) -> (Vec<f32>, f32, usize) {
    let mut baselines = Vec::new();
    let mut text_height = 0.0;
    for (index, line) in lines.iter().enumerate() {
        let vertical = line_box(line, style);
        let mut line_height = jasper_line_height(index == 0, style, &vertical);
        if index == 0 {
            line_height += style.spacing_before.unwrap_or(0.0);
        }
        if text_height + line_height + vertical.descent > height + 0.01 {
            break;
        }
        baselines.push(text_height + line_height);
        text_height += line_height + vertical.descent;
    }
    let fitted = baselines.len();
    (baselines, text_height, fitted)
}

fn line_box(line: &WrappedLine, style: &TextStyle) -> FontVertical {
    let mut merged: Option<FontVertical> = None;
    for segment in &line.segments {
        for word in &segment.words {
            let vertical = font_vertical(run_bytes(&word.style), word.style.size);
            merged = Some(match merged {
                Some(current) => FontVertical {
                    ascent: current.ascent.max(vertical.ascent),
                    descent: current.descent.max(vertical.descent),
                    leading: current.leading.max(vertical.leading),
                    underline: current.underline,
                    strike: current.strike,
                },
                None => vertical,
            });
        }
    }
    merged.unwrap_or_else(|| font_vertical(font_bytes(style), style.font_size.unwrap_or(10.0)))
}

fn layout_lines(text: &str, style: &TextStyle, width: f32, pad_left: f32) -> Vec<WrappedLine> {
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let base = RunStyle::base(style);
    let chunks = match style.markup {
        Markup::None => vec![(normalized, base)],
        Markup::Styled | Markup::Html => parse_markup(&normalized, &base),
    };
    let paragraphs = split_paragraphs(&chunks);
    let left = style.left_indent.unwrap_or(0.0);
    let right_limit = (width - style.right_indent.unwrap_or(0.0)).max(0.0);
    let first_indent = style.first_line_indent.unwrap_or(0.0);
    let right_element = pad_left + right_limit;
    let stops = tab_stops(style, right_element);
    let mut lines = Vec::new();
    for paragraph in paragraphs {
        if paragraph.is_empty() || paragraph.iter().all(|(text, _)| text.is_empty()) {
            lines.push(WrappedLine {
                segments: vec![Segment {
                    words: Vec::new(),
                    width: 0.0,
                    start: left,
                }],
                paragraph_end: true,
            });
            continue;
        }
        let tokens = tokenize(&paragraph);
        let mut segment_words: Vec<Word> = Vec::new();
        let mut segment_width = 0.0;
        let mut segments: Vec<Segment> = Vec::new();
        let mut first_of_paragraph = true;
        let mut tabbed: Option<(f32, TabStopAlign)> = None;
        let start_of = |first: bool, tab: Option<(f32, TabStopAlign)>| -> f32 {
            if let Some((position, _)) = tab {
                left + position - pad_left
            } else if first {
                left + first_indent
            } else {
                left
            }
        };
        let room_of = |start: f32, tab: Option<(f32, TabStopAlign)>| match tab {
            Some((_, TabStopAlign::Right)) => (start - left).max(0.0),
            Some((_, TabStopAlign::Center)) => {
                let before = (start - left).max(0.0);
                let after = (right_limit - start).max(0.0);
                before.min(after) * 2.0
            }
            _ => (right_limit - start).max(0.0),
        };
        let flush_segment = |words: &mut Vec<Word>,
                             width: &mut f32,
                             segments: &mut Vec<Segment>,
                             first: bool,
                             tab: Option<(f32, TabStopAlign)>| {
            let mut start = start_of(first, tab);
            if let Some((position, alignment)) = tab {
                let anchor = left + position - pad_left;
                start = match alignment {
                    TabStopAlign::Right => anchor - *width,
                    TabStopAlign::Center => anchor - *width / 2.0,
                    TabStopAlign::Left => anchor,
                };
            }
            segments.push(Segment {
                words: std::mem::take(words),
                width: *width,
                start,
            });
            *width = 0.0;
        };
        let flush_line = |words: &mut Vec<Word>,
                          width: &mut f32,
                          segments: &mut Vec<Segment>,
                          lines: &mut Vec<WrappedLine>,
                          first: &mut bool,
                          tab: &mut Option<(f32, TabStopAlign)>,
                          paragraph_end: bool| {
            flush_segment(words, width, segments, *first, *tab);
            lines.push(WrappedLine {
                segments: std::mem::take(segments),
                paragraph_end,
            });
            *first = false;
            *tab = None;
        };
        for token in tokens {
            match token {
                Token::Tab => {
                    let edge = pad_left + start_of(first_of_paragraph, tabbed) + segment_width;
                    let next = stops.iter().find(|stop| stop.0 > edge + 0.01).copied();
                    flush_segment(
                        &mut segment_words,
                        &mut segment_width,
                        &mut segments,
                        first_of_paragraph,
                        tabbed,
                    );
                    first_of_paragraph = false;
                    tabbed = next;
                }
                Token::Word(text, run_style) => {
                    let start = start_of(first_of_paragraph, tabbed);
                    let room = room_of(start, tabbed);
                    let pieces = break_word(&text, room, &|sample| {
                        measure(run_bytes(&run_style), sample, run_style.size)
                    });
                    let piece_count = pieces.len();
                    for (piece_index, piece) in pieces.into_iter().enumerate() {
                        let piece_width = measure(run_bytes(&run_style), &piece, run_style.size);
                        let forced = piece_count > 1 && piece_index + 1 < piece_count;
                        let space = if segment_words.is_empty() {
                            0.0
                        } else {
                            measure(run_bytes(&run_style), " ", run_style.size)
                        };
                        if !segment_words.is_empty()
                            && segment_width + space + piece_width > room + 0.01
                        {
                            flush_line(
                                &mut segment_words,
                                &mut segment_width,
                                &mut segments,
                                &mut lines,
                                &mut first_of_paragraph,
                                &mut tabbed,
                                false,
                            );
                        }
                        let space = if segment_words.is_empty() {
                            0.0
                        } else {
                            measure(run_bytes(&run_style), " ", run_style.size)
                        };
                        segment_width += space + piece_width;
                        segment_words.push(Word {
                            text: piece,
                            width: piece_width,
                            style: run_style.clone(),
                        });
                        if forced {
                            flush_line(
                                &mut segment_words,
                                &mut segment_width,
                                &mut segments,
                                &mut lines,
                                &mut first_of_paragraph,
                                &mut tabbed,
                                false,
                            );
                        }
                    }
                }
            }
        }
        flush_line(
            &mut segment_words,
            &mut segment_width,
            &mut segments,
            &mut lines,
            &mut first_of_paragraph,
            &mut tabbed,
            true,
        );
    }
    lines
}

enum Token {
    Word(String, RunStyle),
    Tab,
}

fn tokenize(paragraph: &[(String, RunStyle)]) -> Vec<Token> {
    let mut tokens = Vec::new();
    for (text, style) in paragraph {
        let mut word = String::new();
        let flush = |word: &mut String, tokens: &mut Vec<Token>| {
            if !word.is_empty() {
                tokens.push(Token::Word(std::mem::take(word), style.clone()));
            }
        };
        for ch in text.chars() {
            if ch == '\t' {
                flush(&mut word, &mut tokens);
                tokens.push(Token::Tab);
            } else if ch == ' ' {
                flush(&mut word, &mut tokens);
            } else {
                word.push(ch);
            }
        }
        flush(&mut word, &mut tokens);
    }
    tokens
}

fn split_paragraphs(chunks: &[(String, RunStyle)]) -> Vec<Vec<(String, RunStyle)>> {
    let mut paragraphs = vec![Vec::new()];
    for (text, style) in chunks {
        let mut rest = text.as_str();
        while let Some((line, after)) = rest.split_once('\n') {
            if !line.is_empty() {
                paragraphs
                    .last_mut()
                    .unwrap()
                    .push((line.to_string(), style.clone()));
            }
            paragraphs.push(Vec::new());
            rest = after;
        }
        if !rest.is_empty() {
            paragraphs
                .last_mut()
                .unwrap()
                .push((rest.to_string(), style.clone()));
        }
    }
    paragraphs
}

fn tab_stops(style: &TextStyle, right_element: f32) -> Vec<(f32, TabStopAlign)> {
    let mut stops: Vec<(f32, TabStopAlign)> = style
        .tab_stops
        .iter()
        .filter(|stop| stop.position <= right_element + 0.01)
        .map(|stop| (stop.position, stop.alignment))
        .collect();
    stops.sort_by(|left, right| {
        left.0
            .partial_cmp(&right.0)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let step = style.tab_stop_width.unwrap_or(40.0).max(1.0);
    let mut last = if style.tab_stops.is_empty() {
        0.0
    } else {
        stops.last().map(|stop| stop.0).unwrap_or(0.0)
    };
    while last + step <= right_element + 0.01 {
        last = (last / step + 1.0) * step;
        stops.push((last, TabStopAlign::Left));
    }
    stops
}

fn parse_markup(text: &str, base: &RunStyle) -> Vec<(String, RunStyle)> {
    let mut out = Vec::new();
    let mut stack = vec![("".to_string(), base.clone())];
    let mut buf = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut index = 0;
    while index < chars.len() {
        if chars[index] == '&' {
            if let Some((ch, next)) = decode_entity(&chars[index..]) {
                buf.push(ch);
                index += next;
                continue;
            }
        }
        if chars[index] != '<' {
            buf.push(chars[index]);
            index += 1;
            continue;
        }
        let Some((tag, next)) = parse_tag(&chars[index..]) else {
            buf.push('<');
            index += 1;
            continue;
        };
        flush_markup(&mut buf, &stack, &mut out);
        if tag.closing {
            if let Some(position) = stack
                .iter()
                .rposition(|(name, _)| name.eq_ignore_ascii_case(&tag.name))
            {
                stack.truncate(position);
            }
        } else if tag.name.eq_ignore_ascii_case("br") {
            out.push(("\n".to_string(), stack.last().unwrap().1.clone()));
        } else if !tag.self_closing {
            let parent = stack.last().unwrap().1.clone();
            let mut next_style = apply_tag(&parent, &tag);
            if tag.name.eq_ignore_ascii_case("li") {
                out.push(("\n".to_string(), parent));
            }
            if tag.name.eq_ignore_ascii_case("sup") {
                let size = next_style.size;
                next_style.size = (size * 0.65).max(1.0);
                next_style.shift -= size * 0.35;
            } else if tag.name.eq_ignore_ascii_case("sub") {
                let size = next_style.size;
                next_style.size = (size * 0.65).max(1.0);
                next_style.shift += size * 0.15;
            }
            stack.push((tag.name, next_style));
        }
        index += next;
    }
    flush_markup(&mut buf, &stack, &mut out);
    out
}

fn flush_markup(buf: &mut String, stack: &[(String, RunStyle)], out: &mut Vec<(String, RunStyle)>) {
    if buf.is_empty() {
        return;
    }
    out.push((
        std::mem::take(buf),
        stack.last().map(|(_, style)| style.clone()).unwrap(),
    ));
}

struct Tag {
    name: String,
    closing: bool,
    self_closing: bool,
    attrs: Vec<(String, String)>,
}

fn parse_tag(chars: &[char]) -> Option<(Tag, usize)> {
    if chars.first() != Some(&'<') {
        return None;
    }
    let mut index = 1;
    let closing = chars.get(index) == Some(&'/');
    if closing {
        index += 1;
    }
    let name_start = index;
    while index < chars.len() && chars[index].is_ascii_alphanumeric() {
        index += 1;
    }
    if index == name_start {
        return None;
    }
    let name: String = chars[name_start..index].iter().collect();
    let mut attrs = Vec::new();
    loop {
        while index < chars.len() && chars[index].is_whitespace() {
            index += 1;
        }
        if index >= chars.len() {
            return None;
        }
        if chars[index] == '>' {
            index += 1;
            return Some((
                Tag {
                    name,
                    closing,
                    self_closing: false,
                    attrs,
                },
                index,
            ));
        }
        if chars[index] == '/' && chars.get(index + 1) == Some(&'>') {
            return Some((
                Tag {
                    name,
                    closing,
                    self_closing: true,
                    attrs,
                },
                index + 2,
            ));
        }
        let key_start = index;
        while index < chars.len() && (chars[index].is_ascii_alphanumeric() || chars[index] == '-') {
            index += 1;
        }
        if index == key_start {
            return None;
        }
        let key: String = chars[key_start..index].iter().collect();
        while index < chars.len() && chars[index].is_whitespace() {
            index += 1;
        }
        if chars.get(index) != Some(&'=') {
            return None;
        }
        index += 1;
        while index < chars.len() && chars[index].is_whitespace() {
            index += 1;
        }
        let quote = *chars.get(index)?;
        if quote != '"' && quote != '\'' {
            return None;
        }
        index += 1;
        let value_start = index;
        while index < chars.len() && chars[index] != quote {
            index += 1;
        }
        if index >= chars.len() {
            return None;
        }
        let value: String = chars[value_start..index].iter().collect();
        index += 1;
        attrs.push((key, value));
    }
}

fn apply_tag(parent: &RunStyle, tag: &Tag) -> RunStyle {
    let mut style = parent.clone();
    let flag = |value: &str| value.eq_ignore_ascii_case("true") || value == "1";
    let name = tag.name.to_ascii_lowercase();
    match name.as_str() {
        "b" | "strong" => style.bold = true,
        "i" | "em" => style.italic = true,
        "u" => style.underline = true,
        "s" | "strike" | "del" => style.strike = true,
        _ => {}
    }
    for (key, value) in &tag.attrs {
        match key.to_ascii_lowercase().as_str() {
            "isbold" => style.bold = flag(value),
            "isitalic" => style.italic = flag(value),
            "isunderline" => style.underline = flag(value),
            "isstrikethrough" => style.strike = flag(value),
            "size" => {
                if let Ok(size) = value.parse::<f32>() {
                    let html_scale =
                        name == "font" && (1.0..=7.0).contains(&size) && size.fract() == 0.0;
                    style.size = if html_scale {
                        [8.0, 10.0, 12.0, 14.0, 18.0, 24.0, 36.0][size as usize - 1]
                    } else {
                        size.max(1.0)
                    };
                }
            }
            "forecolor" | "color" => {
                if let Some(color) = Color::parse(value) {
                    style.color = color;
                }
            }
            "backcolor" => {
                if let Some(color) = Color::parse(value) {
                    style.backcolor = Some(color);
                }
            }
            _ => {}
        }
    }
    style
}

fn decode_entity(chars: &[char]) -> Option<(char, usize)> {
    if chars.first() != Some(&'&') {
        return None;
    }
    let end = chars.iter().position(|ch| *ch == ';')?;
    let body: String = chars[1..end].iter().collect();
    let ch = match body.as_str() {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" => '\'',
        "nbsp" => ' ',
        other if other.starts_with('#') => {
            let code = if let Some(hex) = other
                .strip_prefix("#x")
                .or_else(|| other.strip_prefix("#X"))
            {
                u32::from_str_radix(hex, 16).ok()?
            } else {
                other[1..].parse().ok()?
            };
            char::from_u32(code)?
        }
        _ => return None,
    };
    Some((ch, end + 1))
}

fn break_word(word: &str, inner: f32, measure: &dyn Fn(&str) -> f32) -> Vec<String> {
    if inner <= 0.0 || measure(word) <= inner + 0.01 {
        return vec![word.to_string()];
    }
    let mut pieces = Vec::new();
    let mut current = String::new();
    let mut width = 0.0;
    for ch in word.chars() {
        let piece = ch.to_string();
        let ch_width = measure(&piece);
        if !current.is_empty() && width + ch_width > inner + 0.01 {
            pieces.push(std::mem::take(&mut current));
            width = 0.0;
        }
        current.push(ch);
        width += ch_width;
    }
    if !current.is_empty() {
        pieces.push(current);
    }
    pieces
}

fn paint_decoration(
    surface: &mut krilla::surface::Surface<'_>,
    x: f32,
    baseline: f32,
    width: f32,
    decoration: (f32, f32),
    color: Color,
) {
    let (offset, thickness) = decoration;
    stroke_line(
        surface,
        x,
        baseline + offset,
        x + width,
        baseline + offset,
        Some(&Pen {
            line_width: Some(thickness),
            line_color: Some(color),
            line_style: Some("Solid".to_string()),
        }),
    );
}

fn measure(bytes: &[u8], text: &str, size: f32) -> f32 {
    let Ok(font) = FontRef::from_index(bytes, 0) else {
        return 0.0;
    };
    let metrics = font.glyph_metrics(Size::unscaled(), LocationRef::default());
    let charmap = font.charmap();
    let mut units = 0.0;
    for ch in text.chars() {
        let Some(glyph) = charmap.map(ch) else {
            continue;
        };
        units += metrics.advance_width(glyph).unwrap_or(0.0);
    }
    let units_per_em = font.head().map(|head| head.units_per_em()).unwrap_or(1000);
    units * size / f32::from(units_per_em)
}

fn fill_rect(
    surface: &mut krilla::surface::Surface<'_>,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    color: Color,
    alpha: u8,
) {
    let Some(path) = rect_path(x, y, width, height) else {
        return;
    };
    surface.set_stroke(None);
    surface.set_fill(Some(fill_alpha(color, alpha)));
    surface.draw_path(&path);
}

fn stroke_box(
    surface: &mut krilla::surface::Surface<'_>,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    borders: &Box,
) {
    let top = borders.top.as_ref().or(borders.pen.as_ref());
    let right = borders.right.as_ref().or(borders.pen.as_ref());
    let bottom = borders.bottom.as_ref().or(borders.pen.as_ref());
    let left = borders.left.as_ref().or(borders.pen.as_ref());
    // Same pen on every side: one closed path so joins anti-alias cleanly.
    if let (Some(t), Some(r), Some(b), Some(l)) = (top, right, bottom, left) {
        if pens_match(t, r) && pens_match(t, b) && pens_match(t, l) {
            stroke_rect(surface, x, y, width, height, Some(t));
            return;
        }
    }
    let sides = [
        (top, (x, y, x + width, y)),
        (right, (x + width, y, x + width, y + height)),
        (bottom, (x, y + height, x + width, y + height)),
        (left, (x, y, x, y + height)),
    ];
    for (pen, (x1, y1, x2, y2)) in sides {
        if let Some(pen) = pen {
            stroke_line(surface, x1, y1, x2, y2, Some(pen));
        }
    }
}

fn pens_match(a: &Pen, b: &Pen) -> bool {
    a.line_width == b.line_width
        && a.line_color == b.line_color
        && a.line_style.as_deref().unwrap_or("Solid") == b.line_style.as_deref().unwrap_or("Solid")
}

fn smooth_stroke(width: f32, color: Color) -> Stroke {
    Stroke {
        paint: rgb::Color::new(color.r, color.g, color.b).into(),
        width,
        line_cap: LineCap::Round,
        line_join: LineJoin::Round,
        ..Stroke::default()
    }
}

fn paint_image(
    surface: &mut krilla::surface::Surface<'_>,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    prepared: &crate::image::Prepared,
) {
    match prepared {
        crate::image::Prepared::Blank => {}
        crate::image::Prepared::Icon => paint_missing_icon(surface, x, y, width, height),
        crate::image::Prepared::Bitmap(bitmap) => {
            let Some(path) = rect_path(x, y, width, height) else {
                return;
            };
            let Some(image) = crate::image::krilla_image(&bitmap.bytes) else {
                return;
            };
            let Some(size) = krilla::geom::Size::from_wh(bitmap.width, bitmap.height) else {
                return;
            };
            surface.push_clip_path(&path, &FillRule::NonZero);
            if let Some(rotation) = bitmap.rotation {
                surface.push_transform(&crate::image::rotation_transform(
                    rotation, x, y, width, height,
                ));
            }
            surface.push_transform(&Transform::from_translate(x + bitmap.x, y + bitmap.y));
            surface.draw_image(image, size);
            surface.pop();
            if bitmap.rotation.is_some() {
                surface.pop();
            }
            surface.pop();
        }
    }
}

fn paint_missing_icon(
    surface: &mut krilla::surface::Surface<'_>,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) {
    let side = width.min(height).min(16.0).max(0.0);
    if side <= 0.0 {
        return;
    }
    let left = x + (width - side) / 2.0;
    let top = y + (height - side) / 2.0;
    fill_rect(
        surface,
        left,
        top,
        side,
        side,
        Color {
            r: 226,
            g: 232,
            b: 240,
        },
        255,
    );
    let pen = Pen {
        line_width: Some(1.0),
        line_color: Some(Color {
            r: 100,
            g: 116,
            b: 139,
        }),
        line_style: Some("Solid".to_string()),
    };
    stroke_rect(surface, left, top, side, side, Some(&pen));
    stroke_line(
        surface,
        left + 3.0,
        top + 3.0,
        left + side - 3.0,
        top + side - 3.0,
        Some(&pen),
    );
    stroke_line(
        surface,
        left + side - 3.0,
        top + 3.0,
        left + 3.0,
        top + side - 3.0,
        Some(&pen),
    );
}

fn paint_ellipse(
    surface: &mut krilla::surface::Surface<'_>,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    element: &ReportElement,
) {
    let Some(path) = ellipse_path(x, y, width, height) else {
        return;
    };
    if element.mode == Some(Mode::Opaque) {
        let color = element.backcolor.unwrap_or(Color {
            r: 255,
            g: 255,
            b: 255,
        });
        surface.set_stroke(None);
        surface.set_fill(Some(fill_alpha(color, 255)));
        surface.draw_path(&path);
    }
    let pen = element.borders.as_ref().and_then(|borders| {
        borders
            .pen
            .as_ref()
            .or(borders.top.as_ref())
            .or(borders.left.as_ref())
            .or(borders.bottom.as_ref())
            .or(borders.right.as_ref())
    });
    stroke_path(surface, &path, pen);
}

/// Four cubic segments. Kappa is the circle-to-bezier constant.
fn ellipse_path(x: f32, y: f32, width: f32, height: f32) -> Option<krilla::geom::Path> {
    const KAPPA: f32 = 0.55228475;
    let rx = width / 2.0;
    let ry = height / 2.0;
    if rx <= 0.0 || ry <= 0.0 {
        return None;
    }
    let cx = x + rx;
    let cy = y + ry;
    let kx = KAPPA * rx;
    let ky = KAPPA * ry;
    let mut builder = PathBuilder::new();
    builder.move_to(cx + rx, cy);
    builder.cubic_to(cx + rx, cy + ky, cx + kx, cy + ry, cx, cy + ry);
    builder.cubic_to(cx - kx, cy + ry, cx - rx, cy + ky, cx - rx, cy);
    builder.cubic_to(cx - rx, cy - ky, cx - kx, cy - ry, cx, cy - ry);
    builder.cubic_to(cx + kx, cy - ry, cx + rx, cy - ky, cx + rx, cy);
    builder.close();
    builder.finish()
}

fn stroke_path(
    surface: &mut krilla::surface::Surface<'_>,
    path: &krilla::geom::Path,
    pen: Option<&Pen>,
) {
    if let Some(style) = pen.and_then(|pen| pen.line_style.as_deref()) {
        if style != "Solid" {
            return;
        }
    }
    let width = pen.and_then(|pen| pen.line_width).unwrap_or(0.0);
    if width <= 0.0 {
        return;
    }
    let color = pen
        .and_then(|pen| pen.line_color)
        .unwrap_or(Color { r: 0, g: 0, b: 0 });
    surface.set_fill(None);
    surface.set_stroke(Some(smooth_stroke(width, color)));
    surface.draw_path(path);
}

fn stroke_rect(
    surface: &mut krilla::surface::Surface<'_>,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    pen: Option<&Pen>,
) {
    if let Some(style) = pen.and_then(|pen| pen.line_style.as_deref()) {
        if style != "Solid" {
            return;
        }
    }
    let Some(path) = rect_path(x, y, width, height) else {
        return;
    };
    let width = pen.and_then(|pen| pen.line_width).unwrap_or(0.0);
    if width <= 0.0 {
        return;
    }
    let color = pen
        .and_then(|pen| pen.line_color)
        .unwrap_or(Color { r: 0, g: 0, b: 0 });
    surface.set_fill(None);
    surface.set_stroke(Some(smooth_stroke(width, color)));
    surface.draw_path(&path);
}

fn stroke_line(
    surface: &mut krilla::surface::Surface<'_>,
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    pen: Option<&Pen>,
) {
    if let Some(style) = pen.and_then(|pen| pen.line_style.as_deref()) {
        if style != "Solid" {
            return;
        }
    }
    let mut builder = PathBuilder::new();
    builder.move_to(x1, y1);
    builder.line_to(x2, y2);
    let Some(path) = builder.finish() else {
        return;
    };
    let width = pen.and_then(|pen| pen.line_width).unwrap_or(0.0);
    if width <= 0.0 {
        return;
    }
    let color = pen
        .and_then(|pen| pen.line_color)
        .unwrap_or(Color { r: 0, g: 0, b: 0 });
    surface.set_fill(None);
    surface.set_stroke(Some(smooth_stroke(width, color)));
    surface.draw_path(&path);
}

fn fill(color: Color) -> Fill {
    fill_alpha(color, 255)
}

fn fill_alpha(color: Color, alpha: u8) -> Fill {
    let opacity = NormalizedF32::new(alpha as f32 / 255.0).unwrap_or(NormalizedF32::ONE);
    Fill {
        paint: rgb::Color::new(color.r, color.g, color.b).into(),
        opacity,
        rule: FillRule::NonZero,
    }
}

fn rect_path(x: f32, y: f32, width: f32, height: f32) -> Option<krilla::geom::Path> {
    let rect = Rect::from_xywh(x, y, width, height)?;
    let mut builder = PathBuilder::new();
    builder.push_rect(rect);
    builder.finish()
}

fn paint_chart(
    surface: &mut krilla::surface::Surface<'_>,
    origin_x: f32,
    origin_y: f32,
    record: &pdfrost_charts::Record,
    regular: &Font,
    bold: &Font,
) {
    for mark in &record.paints {
        match mark {
            pdfrost_charts::Paint::FillRect {
                x,
                y,
                width,
                height,
                color,
            } => fill_rect(
                surface,
                origin_x + x,
                origin_y + y,
                *width,
                *height,
                chart_color(*color),
                color.a,
            ),
            pdfrost_charts::Paint::StrokeRect {
                x,
                y,
                width,
                height,
                color,
                stroke,
            } => {
                let pen = Pen {
                    line_width: Some(*stroke),
                    line_color: Some(chart_color(*color)),
                    line_style: Some("Solid".to_string()),
                };
                stroke_rect(
                    surface,
                    origin_x + x,
                    origin_y + y,
                    *width,
                    *height,
                    Some(&pen),
                );
            }
            pdfrost_charts::Paint::Polygon { points, color } => {
                fill_polygon(
                    surface,
                    origin_x,
                    origin_y,
                    points,
                    chart_color(*color),
                    color.a,
                );
            }
            pdfrost_charts::Paint::Line {
                x1,
                y1,
                x2,
                y2,
                color,
                stroke,
            } => {
                let pen = Pen {
                    line_width: Some(*stroke),
                    line_color: Some(chart_color(*color)),
                    line_style: Some("Solid".to_string()),
                };
                stroke_line(
                    surface,
                    origin_x + x1,
                    origin_y + y1,
                    origin_x + x2,
                    origin_y + y2,
                    Some(&pen),
                );
            }
            pdfrost_charts::Paint::Text {
                x,
                y,
                size,
                bold: is_bold,
                color,
                text,
            } => {
                if text.is_empty() {
                    continue;
                }
                let font = if *is_bold { bold } else { regular };
                surface.set_stroke(None);
                surface.set_fill(Some(fill_alpha(chart_color(*color), color.a)));
                surface.draw_text(
                    Point::from_xy(origin_x + x, origin_y + y),
                    font.clone(),
                    *size,
                    text,
                    false,
                    TextDirection::Auto,
                );
            }
        }
    }
}

fn fill_polygon(
    surface: &mut krilla::surface::Surface<'_>,
    origin_x: f32,
    origin_y: f32,
    points: &[(f32, f32)],
    color: Color,
    alpha: u8,
) {
    if points.len() < 3 {
        return;
    }
    let mut builder = PathBuilder::new();
    builder.move_to(origin_x + points[0].0, origin_y + points[0].1);
    for (x, y) in points.iter().skip(1) {
        builder.line_to(origin_x + x, origin_y + y);
    }
    builder.close();
    let Some(path) = builder.finish() else {
        return;
    };
    surface.set_stroke(None);
    surface.set_fill(Some(fill_alpha(color, alpha)));
    surface.draw_path(&path);
}

fn chart_color(color: pdfrost_charts::Rgb) -> Color {
    Color {
        r: color.r,
        g: color.g,
        b: color.b,
    }
}

#[cfg(test)]
mod text_layout {
    use super::*;

    fn style(align: Option<TextAlignment>, vertical: Option<VerticalAlignment>) -> TextStyle {
        TextStyle {
            text_alignment: align,
            vertical_alignment: vertical,
            font_size: Some(10.0),
            ..TextStyle::default()
        }
    }

    fn runs(text: &str, style: &TextStyle, width: f32, height: f32) -> Vec<TextRun> {
        place_text(text, style, 0.0, 0.0, width, height, 0.0)
    }

    #[test]
    fn a_newline_starts_another_left_aligned_line() {
        let laid = runs(
            "Transfer Note\nInter Locator Transfer (ILT)",
            &style(Some(TextAlignment::Left), Some(VerticalAlignment::Top)),
            180.0,
            30.0,
        );
        assert_eq!(laid.len(), 2);
        assert_eq!(laid[0].text, "Transfer Note");
        assert_eq!(laid[1].text, "Inter Locator Transfer (ILT)");
        assert_eq!(laid[0].x, 0.0);
        assert_eq!(laid[1].x, 0.0);
        assert!(laid[1].y > laid[0].y);
    }

    #[test]
    fn words_wrap_inside_the_box_and_a_short_line_stays_centered() {
        let font = crate::DEFAULT_SANS_REGULAR;
        let word = measure(font, "Item", 10.0).max(measure(font, "Number", 10.0));
        let laid = runs(
            "Item Number",
            &style(Some(TextAlignment::Center), Some(VerticalAlignment::Middle)),
            word + 1.0,
            40.0,
        );
        assert_eq!(laid.len(), 2);
        assert_eq!(laid[0].text, "Item");
        assert_eq!(laid[1].text, "Number");
        assert!(laid[0].x > 0.0);
        assert!(laid[1].x > 0.0);
    }

    #[test]
    fn justified_text_spreads_a_full_line_and_leaves_the_last_line_on_the_left() {
        let font = crate::DEFAULT_SANS_REGULAR;
        let one = measure(font, "One", 10.0);
        let two = measure(font, "Two", 10.0);
        let space = measure(font, " ", 10.0);
        let inner = one + space + two + 20.0;
        let laid = runs(
            "One Two Three",
            &style(Some(TextAlignment::Justified), None),
            inner,
            40.0,
        );
        assert_eq!(laid.len(), 3);
        assert_eq!(laid[0].text, "One");
        assert_eq!(laid[1].text, "Two");
        assert!((laid[1].x - (one + space + 20.0)).abs() < 0.2);
        assert_eq!(laid[2].text, "Three");
        assert_eq!(laid[2].x, 0.0);
    }

    #[test]
    fn a_left_indent_insets_every_line() {
        let mut indented = style(Some(TextAlignment::Left), None);
        indented.left_indent = Some(8.0);
        let laid = runs("From\nLocator", &indented, 180.0, 40.0);
        assert_eq!(laid.len(), 2);
        assert_eq!(laid[0].x, 8.0);
        assert_eq!(laid[1].x, 8.0);
    }

    #[test]
    fn double_spacing_keeps_the_first_line_single_and_doubles_the_ascent_after_it() {
        let mut spaced = style(Some(TextAlignment::Left), Some(VerticalAlignment::Top));
        spaced.line_spacing = Some(LineSpacing::Double);
        let laid = runs("One\nTwo", &spaced, 180.0, 80.0);
        let vertical = font_vertical(crate::DEFAULT_SANS_REGULAR, 10.0);
        let first = vertical.leading + vertical.ascent;
        let gap = vertical.descent + vertical.leading + 2.0 * vertical.ascent;
        assert_eq!(laid.len(), 2);
        assert!((laid[0].y - first).abs() < 0.05);
        assert!((laid[1].y - laid[0].y - gap).abs() < 0.05);
    }

    #[test]
    fn the_first_line_indent_does_not_apply_to_a_wrapped_line() {
        let font = crate::DEFAULT_SANS_REGULAR;
        let from = measure(font, "From", 10.0);
        let locator = measure(font, "Locator", 10.0);
        let space = measure(font, " ", 10.0);
        let mut indented = style(Some(TextAlignment::Left), None);
        indented.first_line_indent = Some(12.0);
        let laid = runs("From Locator", &indented, from + space + locator, 40.0);
        assert_eq!(laid[0].text, "From");
        assert_eq!(laid[0].x, 12.0);
        assert_eq!(laid[1].text, "Locator");
        assert_eq!(laid[1].x, 0.0);
    }

    #[test]
    fn spacing_before_lowers_only_the_block() {
        let mut spaced = style(Some(TextAlignment::Left), Some(VerticalAlignment::Top));
        spaced.spacing_before = Some(6.0);
        let plain = runs(
            "One",
            &style(Some(TextAlignment::Left), Some(VerticalAlignment::Top)),
            80.0,
            40.0,
        );
        let laid = runs("One", &spaced, 80.0, 40.0);
        assert!((laid[0].y - plain[0].y - 6.0).abs() < 0.05);
    }

    #[test]
    fn a_tab_moves_the_next_word_to_the_default_stop() {
        let laid = runs("A\tB", &style(Some(TextAlignment::Left), None), 200.0, 40.0);
        assert_eq!(laid.len(), 2);
        assert_eq!(laid[0].text, "A");
        assert_eq!(laid[1].text, "B");
        assert!((laid[1].x - 40.0).abs() < 0.05);
    }

    #[test]
    fn strike_through_is_carried_on_the_run() {
        let mut struck = style(None, None);
        struck.strike_through = true;
        let laid = runs("Cut", &struck, 80.0, 20.0);
        assert!(laid[0].style.strike);
    }

    #[test]
    fn styled_markup_makes_one_word_bold_and_plain_markup_stays_text() {
        let mut marked = style(Some(TextAlignment::Left), None);
        marked.markup = Markup::Styled;
        let laid = runs("Say <b>Hi</b>", &marked, 180.0, 20.0);
        assert_eq!(laid.len(), 2);
        assert_eq!(laid[0].text, "Say");
        assert!(!laid[0].style.bold);
        assert_eq!(laid[1].text, "Hi");
        assert!(laid[1].style.bold);
        assert!(laid[1].x > laid[0].x);
        let plain = runs(
            "Say <b>Hi</b>",
            &style(Some(TextAlignment::Left), None),
            180.0,
            20.0,
        );
        assert_eq!(plain.len(), 1);
        assert_eq!(plain[0].text, "Say <b>Hi</b>");
    }

    #[test]
    fn left_rotation_starts_at_the_bottom_of_the_box() {
        let transform = text_transform(Some(Rotation::Left), 10.0, 20.0, 30.0, 40.0);
        assert_eq!(transform.tx(), 10.0);
        assert_eq!(transform.ty(), 60.0);
        assert_eq!(transform.kx(), 1.0);
        assert_eq!(transform.ky(), -1.0);
    }
}
