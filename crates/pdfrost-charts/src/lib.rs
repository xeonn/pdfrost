//! Category charts drawn in the Chart.js 4.4.6 color scheme.
//!
//! The layout still follows a JasperReports category plot: title, ticks, legend,
//! and category slots. The colors are Chart.js defaults. Series borders walk the
//! colors-plugin palette. Bars and areas use that color at 50% alpha. The plot
//! is white, gridlines and the axis frame are `rgba(0, 0, 0, 0.1)`, and the
//! title, ticks, and legend are `#666`.
//!
//! The font bytes come from the caller. pdfrost passes Liberation Sans, the
//! face the rest of the page already embeds. Title, tick, and legend sizes stay
//! at 20 pt bold and 12 pt.
//!
//! Nothing here writes a PDF. [`draw`] emits marks through [`Canvas`]. The
//! engine implements that trait on a krilla surface. The split follows
//! plotters, whose `DrawingBackend` is separate from the chart, and charts-rs,
//! which turns one chart value into one picture. Those crates keep their own
//! themes and their own fonts, so they are a structure reference only.

mod canvas;
mod color;
mod dataset;
mod draw;
mod font;

pub use canvas::{Canvas, Paint, Record};
pub use color::{series_color, series_fill, Rgb, CHART_BACKGROUND, GRID, TEXT};
pub use dataset::CategoryDataset;
pub use draw::{draw, CategoryChart, ChartKind};
pub use font::{Fonts, SkrifaFace};
