use crate::canvas::Canvas;
use crate::color::{series_color, series_fill, CHART_BACKGROUND, GRID, TEXT};
use crate::dataset::CategoryDataset;
use crate::font::Fonts;

/// Which category renderer to use.
///
/// The names match the JRXML element tags this plot stands in for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartKind {
    Area,
    Bar,
    Line,
    StackedArea,
    StackedBar,
}

impl ChartKind {
    fn stacked(self) -> bool {
        matches!(self, Self::StackedArea | Self::StackedBar)
    }
}

/// One category chart, ready to paint into a box of `width` by `height` points.
#[derive(Debug, Clone, PartialEq)]
pub struct CategoryChart {
    pub kind: ChartKind,
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub show_legend: bool,
    pub dataset: CategoryDataset,
}

/// JFreeChart 1.0.19 `StandardChartTheme` sizes. The face is the caller's.
const TITLE_SIZE: f32 = 20.0;
const REGULAR_SIZE: f32 = 12.0;

/// `CategoryAxis` defaults: lower margin, upper margin, category margin.
const LOWER_MARGIN: f32 = 0.05;
const UPPER_MARGIN: f32 = 0.05;
const CATEGORY_MARGIN: f32 = 0.20;

/// `BarRenderer` default item margin.
const ITEM_MARGIN: f32 = 0.20;

/// `StandardChartTheme.axisOffset`, four points on every side of the plot.
const AXIS_OFFSET: f32 = 4.0;
/// Chart.js `elements.line` `borderWidth`.
const LINE_WIDTH: f32 = 3.0;
/// Chart.js point diameter (`radius` 3).
const POINT_SIZE: f32 = 6.0;

/// Paint `chart` into a box of `width` by `height` points.
///
/// The origin is the top-left of that box. `fonts` measures and is the face
/// [`Canvas::text`] should draw with.
pub fn draw(
    chart: &CategoryChart,
    width: f32,
    height: f32,
    fonts: &impl Fonts,
    canvas: &mut impl Canvas,
) {
    if width <= 1.0 || height <= 1.0 {
        return;
    }
    canvas.fill_rect(0.0, 0.0, width, height, CHART_BACKGROUND);

    let mut top = 0.0;
    if let Some(title) = chart.title.as_deref().filter(|text| !text.is_empty()) {
        let text_width = fonts.width(title, TITLE_SIZE, true);
        let baseline = top + fonts.ascent(TITLE_SIZE, true);
        canvas.text(
            (width - text_width) / 2.0,
            baseline,
            TITLE_SIZE,
            true,
            TEXT,
            title,
        );
        top = baseline + 4.0;
    }
    if let Some(subtitle) = chart.subtitle.as_deref().filter(|text| !text.is_empty()) {
        let text_width = fonts.width(subtitle, REGULAR_SIZE, false);
        let baseline = top + fonts.ascent(REGULAR_SIZE, false);
        canvas.text(
            (width - text_width) / 2.0,
            baseline,
            REGULAR_SIZE,
            false,
            TEXT,
            subtitle,
        );
        top = baseline + 2.0;
    }

    let legend = legend_block(chart, fonts);
    let (lower, upper, ticks) = axis_range(&chart.dataset, chart.kind.stacked());
    let tick_labels: Vec<String> = ticks.iter().copied().map(format_tick).collect();
    let left = AXIS_OFFSET
        + tick_labels
            .iter()
            .map(|label| fonts.width(label, REGULAR_SIZE, false))
            .fold(0.0, f32::max)
        + AXIS_OFFSET;
    let bottom_labels = fonts.ascent(REGULAR_SIZE, false) + 2.0;
    let plot_top = top + AXIS_OFFSET;
    let plot_bottom = (height - legend.height - bottom_labels - AXIS_OFFSET).max(plot_top);
    let plot_left = left;
    let plot_right = (width - AXIS_OFFSET).max(plot_left);
    let plot_width = plot_right - plot_left;
    let plot_height = plot_bottom - plot_top;
    if plot_width <= 1.0 || plot_height <= 1.0 {
        paint_legend(chart, fonts, width, height, &legend, canvas);
        return;
    }

    let span = upper - lower;
    for tick in &ticks {
        if (*tick - lower).abs() < f64::EPSILON || (*tick - upper).abs() < f64::EPSILON {
            continue;
        }
        let y = value_y(*tick, lower, span, plot_top, plot_height);
        canvas.stroke_line(plot_left, y, plot_right, y, GRID, 1.0);
    }
    paint_vertical_grid(chart, plot_left, plot_top, plot_width, plot_bottom, canvas);
    canvas.stroke_rect(plot_left, plot_top, plot_width, plot_height, GRID, 1.0);

    let categories = chart.dataset.category_count();
    if categories > 0 && chart.dataset.series_count() > 0 {
        match chart.kind {
            ChartKind::Area => paint_area(
                chart,
                false,
                lower,
                span,
                plot_left,
                plot_top,
                plot_width,
                plot_height,
                canvas,
            ),
            ChartKind::StackedArea => paint_area(
                chart,
                true,
                lower,
                span,
                plot_left,
                plot_top,
                plot_width,
                plot_height,
                canvas,
            ),
            ChartKind::Bar => paint_bars(
                chart,
                false,
                lower,
                span,
                plot_left,
                plot_top,
                plot_width,
                plot_height,
                canvas,
            ),
            ChartKind::StackedBar => paint_bars(
                chart,
                true,
                lower,
                span,
                plot_left,
                plot_top,
                plot_width,
                plot_height,
                canvas,
            ),
            ChartKind::Line => paint_line(
                chart,
                lower,
                span,
                plot_left,
                plot_top,
                plot_width,
                plot_height,
                canvas,
            ),
        }
    }

    for (tick, label) in ticks.iter().zip(tick_labels.iter()) {
        let y = value_y(*tick, lower, span, plot_top, plot_height);
        let text_width = fonts.width(label, REGULAR_SIZE, false);
        let baseline = y + fonts.ascent(REGULAR_SIZE, false) * 0.35;
        canvas.text(
            plot_left - AXIS_OFFSET - text_width,
            baseline,
            REGULAR_SIZE,
            false,
            TEXT,
            label,
        );
    }
    if categories > 0 {
        for index in 0..categories {
            let (start, size) = category_span(index, categories, plot_left, plot_width);
            let label = chart.dataset.category(index);
            let text_width = fonts.width(label, REGULAR_SIZE, false);
            let baseline = plot_bottom + AXIS_OFFSET + fonts.ascent(REGULAR_SIZE, false);
            canvas.text(
                start + (size - text_width) / 2.0,
                baseline,
                REGULAR_SIZE,
                false,
                TEXT,
                label,
            );
        }
    }
    paint_legend(chart, fonts, width, height, &legend, canvas);
}

struct Legend {
    height: f32,
    items: Vec<(String, f32)>,
}

fn legend_block(chart: &CategoryChart, fonts: &impl Fonts) -> Legend {
    if !chart.show_legend || chart.dataset.series_count() == 0 {
        return Legend {
            height: 0.0,
            items: Vec::new(),
        };
    }
    let items = (0..chart.dataset.series_count())
        .map(|index| {
            let name = chart.dataset.series_name(index).to_string();
            let width = 8.0 + 4.0 + fonts.width(&name, REGULAR_SIZE, false);
            (name, width)
        })
        .collect();
    Legend {
        height: REGULAR_SIZE + 8.0,
        items,
    }
}

fn paint_vertical_grid(
    chart: &CategoryChart,
    plot_left: f32,
    plot_top: f32,
    plot_width: f32,
    plot_bottom: f32,
    canvas: &mut impl Canvas,
) {
    let categories = chart.dataset.category_count();
    if categories == 0 {
        return;
    }
    let between = matches!(chart.kind, ChartKind::Bar | ChartKind::StackedBar);
    for index in 0..categories {
        let (start, size) = category_span(index, categories, plot_left, plot_width);
        let x = if between { start } else { start + size / 2.0 };
        canvas.stroke_line(x, plot_top, x, plot_bottom, GRID, 1.0);
    }
    if between {
        let (start, size) = category_span(categories - 1, categories, plot_left, plot_width);
        canvas.stroke_line(start + size, plot_top, start + size, plot_bottom, GRID, 1.0);
    }
}

fn paint_legend(
    chart: &CategoryChart,
    fonts: &impl Fonts,
    width: f32,
    height: f32,
    legend: &Legend,
    canvas: &mut impl Canvas,
) {
    if legend.items.is_empty() {
        return;
    }
    let gap = 12.0;
    let total = legend
        .items
        .iter()
        .map(|(_, item_width)| *item_width)
        .sum::<f32>()
        + gap * (legend.items.len().saturating_sub(1) as f32);
    let mut x = ((width - total) / 2.0).max(0.0);
    let swatch_y = height - legend.height + 4.0;
    let baseline = swatch_y + fonts.ascent(REGULAR_SIZE, false);
    for (index, (name, item_width)) in legend.items.iter().enumerate() {
        let swatch = if chart.kind == ChartKind::Line {
            series_color(index)
        } else {
            series_fill(index)
        };
        canvas.fill_rect(x, swatch_y, 8.0, 8.0, swatch);
        canvas.text(x + 12.0, baseline, REGULAR_SIZE, false, TEXT, name);
        x += item_width + gap;
    }
}

fn paint_area(
    chart: &CategoryChart,
    stacked: bool,
    lower: f64,
    span: f64,
    plot_left: f32,
    plot_top: f32,
    plot_width: f32,
    plot_height: f32,
    canvas: &mut impl Canvas,
) {
    let categories = chart.dataset.category_count();
    let centers: Vec<f32> = (0..categories)
        .map(|index| {
            let (start, size) = category_span(index, categories, plot_left, plot_width);
            start + size / 2.0
        })
        .collect();
    for series in 0..chart.dataset.series_count() {
        let mut points = Vec::with_capacity(categories * 2 + 2);
        for category in 0..categories {
            let below = if stacked {
                (0..series)
                    .map(|earlier| chart.dataset.value(earlier, category))
                    .sum()
            } else {
                0.0
            };
            let above = below + chart.dataset.value(series, category);
            points.push((
                centers[category],
                value_y(above, lower, span, plot_top, plot_height),
            ));
        }
        for category in (0..categories).rev() {
            let below = if stacked {
                (0..series)
                    .map(|earlier| chart.dataset.value(earlier, category))
                    .sum()
            } else {
                0.0
            };
            points.push((
                centers[category],
                value_y(below, lower, span, plot_top, plot_height),
            ));
        }
        canvas.fill_polygon(&points, series_fill(series));
        let color = series_color(series);
        for pair in points[..categories].windows(2) {
            canvas.stroke_line(
                pair[0].0, pair[0].1, pair[1].0, pair[1].1, color, LINE_WIDTH,
            );
        }
    }
}

fn paint_bars(
    chart: &CategoryChart,
    stacked: bool,
    lower: f64,
    span: f64,
    plot_left: f32,
    plot_top: f32,
    plot_width: f32,
    plot_height: f32,
    canvas: &mut impl Canvas,
) {
    let categories = chart.dataset.category_count();
    let series_count = chart.dataset.series_count();
    for category in 0..categories {
        let (start, size) = category_span(category, categories, plot_left, plot_width);
        if stacked {
            let bar_width = size * (1.0 - ITEM_MARGIN);
            let x = start + (size - bar_width) / 2.0;
            let mut floor = 0.0;
            for series in 0..series_count {
                let value = chart.dataset.value(series, category);
                let y0 = value_y(floor, lower, span, plot_top, plot_height);
                let y1 = value_y(floor + value, lower, span, plot_top, plot_height);
                floor += value;
                let top = y0.min(y1);
                let height = (y0 - y1).abs().max(0.5);
                canvas.fill_rect(x, top, bar_width, height, series_fill(series));
            }
        } else {
            let inner = size * (1.0 - ITEM_MARGIN);
            let bar_width = inner / series_count as f32;
            let gap = size * ITEM_MARGIN / (series_count as f32 + 1.0);
            for series in 0..series_count {
                let x = start + gap + series as f32 * (bar_width + gap);
                let value = chart.dataset.value(series, category);
                let y0 = value_y(0.0, lower, span, plot_top, plot_height);
                let y1 = value_y(value, lower, span, plot_top, plot_height);
                let top = y0.min(y1);
                let height = (y0 - y1).abs().max(0.5);
                canvas.fill_rect(x, top, bar_width, height, series_fill(series));
            }
        }
    }
}

fn paint_line(
    chart: &CategoryChart,
    lower: f64,
    span: f64,
    plot_left: f32,
    plot_top: f32,
    plot_width: f32,
    plot_height: f32,
    canvas: &mut impl Canvas,
) {
    let categories = chart.dataset.category_count();
    for series in 0..chart.dataset.series_count() {
        let color = series_color(series);
        let mut previous = None;
        for category in 0..categories {
            let (start, size) = category_span(category, categories, plot_left, plot_width);
            let x = start + size / 2.0;
            let y = value_y(
                chart.dataset.value(series, category),
                lower,
                span,
                plot_top,
                plot_height,
            );
            if let Some((px, py)) = previous {
                canvas.stroke_line(px, py, x, y, color, LINE_WIDTH);
            }
            let point = x - POINT_SIZE / 2.0;
            let point_y = y - POINT_SIZE / 2.0;
            canvas.fill_rect(point, point_y, POINT_SIZE, POINT_SIZE, series_fill(series));
            canvas.stroke_rect(point, point_y, POINT_SIZE, POINT_SIZE, color, 1.0);
            previous = Some((x, y));
        }
    }
}

/// Category slot from `CategoryAxis`: margins come off the length first, and
/// `categoryMargin` is the total gap budget shared between the slots.
fn category_span(index: usize, count: usize, plot_left: f32, plot_width: f32) -> (f32, f32) {
    let count_f = count as f32;
    let available = plot_width * (1.0 - LOWER_MARGIN - UPPER_MARGIN - CATEGORY_MARGIN);
    let size = available / count_f;
    let gap = if count > 1 {
        plot_width * CATEGORY_MARGIN / (count_f - 1.0)
    } else {
        0.0
    };
    let start = plot_left + plot_width * LOWER_MARGIN + index as f32 * (size + gap);
    (start, size)
}

fn value_y(value: f64, lower: f64, span: f64, plot_top: f32, plot_height: f32) -> f32 {
    let t = ((value - lower) / span) as f32;
    plot_top + plot_height - t * plot_height
}

fn axis_range(dataset: &CategoryDataset, stacked: bool) -> (f64, f64, Vec<f64>) {
    let (min_value, max_value) = dataset.peak(stacked);
    let mut lower = min_value.min(0.0);
    let mut upper = max_value.max(0.0);
    if upper - lower < f64::EPSILON {
        upper = lower + 1.0;
    }
    let step = nice_step((upper - lower) / 4.0);
    lower = (lower / step).floor() * step;
    upper = (upper / step).ceil() * step;
    if upper - lower < f64::EPSILON {
        upper = lower + step;
    }
    let mut ticks = Vec::new();
    let mut value = lower;
    let mut guard = 0;
    while value <= upper + step * 0.001 && guard < 12 {
        ticks.push(value);
        value += step;
        guard += 1;
    }
    if ticks.is_empty() {
        ticks.push(0.0);
    }
    (lower, upper, ticks)
}

fn nice_step(raw: f64) -> f64 {
    if raw <= 0.0 || !raw.is_finite() {
        return 1.0;
    }
    let exponent = raw.log10().floor();
    let base = 10f64.powf(exponent);
    let fraction = raw / base;
    let nice = if fraction <= 1.0 {
        1.0
    } else if fraction <= 2.0 {
        2.0
    } else if fraction <= 5.0 {
        5.0
    } else {
        10.0
    };
    nice * base
}

fn format_tick(value: f64) -> String {
    if (value - value.round()).abs() < 1e-6 {
        format!("{}", value.round() as i64)
    } else {
        format!("{value:.1}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::{Paint, Record};
    use crate::color::Rgb;
    use crate::font::SkrifaFace;

    struct UnitFont;

    impl Fonts for UnitFont {
        fn width(&self, text: &str, size: f32, _: bool) -> f32 {
            text.chars().count() as f32 * size * 0.5
        }

        fn ascent(&self, size: f32, _: bool) -> f32 {
            size * 0.8
        }
    }

    fn sample(kind: ChartKind) -> CategoryChart {
        let mut dataset = CategoryDataset::new();
        dataset.add_value("A", "Jan", 1.0);
        dataset.add_value("A", "Feb", 3.0);
        dataset.add_value("B", "Jan", 2.0);
        dataset.add_value("B", "Feb", 1.0);
        CategoryChart {
            kind,
            title: Some("Units".to_string()),
            subtitle: None,
            show_legend: true,
            dataset,
        }
    }

    fn paints(kind: ChartKind) -> Vec<Paint> {
        let mut record = Record::default();
        draw(&sample(kind), 400.0, 300.0, &UnitFont, &mut record);
        record.paints
    }

    #[test]
    fn the_first_series_uses_the_chartjs_palette() {
        assert_eq!(series_color(0), Rgb::new(54, 162, 235));
        assert_eq!(series_color(1), Rgb::new(255, 99, 132));
        assert_eq!(series_fill(0).a, 128);
        assert_eq!(series_color(7), series_color(0));
    }

    #[test]
    fn a_repeated_category_adds_to_the_value() {
        let mut dataset = CategoryDataset::new();
        dataset.add_value("A", "Jan", 1.0);
        dataset.add_value("A", "Jan", 2.5);
        assert_eq!(dataset.value(0, 0), 3.5);
    }

    #[test]
    fn the_area_rises_toward_the_larger_value() {
        let paints = paints(ChartKind::Area);
        let polygon = paints.iter().find_map(|paint| match paint {
            Paint::Polygon { points, color } if *color == series_fill(0) => Some(points.clone()),
            _ => None,
        });
        let points = polygon.expect("area polygon");
        let jan = points[0].1;
        let feb = points[1].1;
        assert!(
            feb < jan,
            "Feb is 3 and Jan is 1, so Feb sits higher: {feb} vs {jan}"
        );
    }

    #[test]
    fn the_taller_bar_is_the_larger_value() {
        let paints = paints(ChartKind::Bar);
        let bars: Vec<_> = paints
            .iter()
            .filter_map(|paint| match paint {
                Paint::FillRect {
                    y, height, color, ..
                } if *color == series_fill(0) => Some((*y, *height)),
                _ => None,
            })
            .collect();
        assert!(bars.len() >= 2, "{bars:?}");
        let jan = bars[0].1;
        let feb = bars[1].1;
        assert!(feb > jan, "Feb's bar is taller: {feb} vs {jan}");
    }

    #[test]
    fn a_stacked_bar_puts_the_second_series_above_the_first() {
        let paints = paints(ChartKind::StackedBar);
        let first_top = paints.iter().find_map(|paint| match paint {
            Paint::FillRect { y, color, .. } if *color == series_fill(0) => Some(*y),
            _ => None,
        });
        let second_bottom = paints.iter().find_map(|paint| match paint {
            Paint::FillRect {
                y, height, color, ..
            } if *color == series_fill(1) => Some(*y + *height),
            _ => None,
        });
        let top = first_top.expect("first series");
        let bottom = second_bottom.expect("second series");
        assert!(
            (top - bottom).abs() < 0.6,
            "second series starts where the first ends: {top} vs {bottom}"
        );
    }

    #[test]
    fn the_line_connects_category_centers() {
        let paints = paints(ChartKind::Line);
        assert!(paints.iter().any(|paint| matches!(
            paint,
            Paint::Line { color, .. } if *color == series_color(0)
        )));
    }

    #[test]
    fn the_legend_names_the_series_and_the_title_is_bold() {
        let paints = paints(ChartKind::Area);
        assert!(paints.iter().any(|paint| matches!(
            paint,
            Paint::Text { text, bold: true, size, .. } if text == "Units" && *size == TITLE_SIZE
        )));
        assert!(paints.iter().any(|paint| matches!(
            paint,
            Paint::Text { text, .. } if text == "A"
        )));
        assert!(paints.iter().any(|paint| matches!(
            paint,
            Paint::Text { text, .. } if text == "B"
        )));
    }

    #[test]
    fn liberation_sans_sets_the_title_width() {
        let regular = include_bytes!("../../pdfrost-engine/fonts/LiberationSans-Regular.ttf");
        let bold = include_bytes!("../../pdfrost-engine/fonts/LiberationSans-Bold.ttf");
        let fonts = SkrifaFace { regular, bold };
        let title = "Units";
        let width = fonts.width(title, TITLE_SIZE, true);
        assert!(width > 20.0, "{width}");
        let mut record = Record::default();
        let mut dataset = CategoryDataset::new();
        dataset.add_value("SERIES 1", "0", 0.0);
        draw(
            &CategoryChart {
                kind: ChartKind::Area,
                title: Some(title.to_string()),
                subtitle: None,
                show_legend: true,
                dataset,
            },
            200.0,
            200.0,
            &fonts,
            &mut record,
        );
        let title_mark = record.paints.iter().find_map(|paint| match paint {
            Paint::Text {
                x,
                text,
                bold: true,
                ..
            } if text == title => Some(*x),
            _ => None,
        });
        let x = title_mark.expect("title");
        let centered = (200.0 - width) / 2.0;
        assert!(
            (x - centered).abs() < 0.5,
            "title x {x} follows the font width {width}"
        );
    }
}
