use crate::color::Rgb;

/// Where a chart paints. Coordinates are points, origin at the top-left of the
/// chart box. Text `y` is the baseline.
pub trait Canvas {
    fn fill_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: Rgb);
    fn stroke_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: Rgb, stroke: f32);
    fn fill_polygon(&mut self, points: &[(f32, f32)], color: Rgb);
    fn stroke_line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, color: Rgb, stroke: f32);
    fn text(&mut self, x: f32, y: f32, size: f32, bold: bool, color: Rgb, text: &str);
}

/// One mark captured by [`Record`].
#[derive(Debug, Clone, PartialEq)]
pub enum Paint {
    FillRect {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: Rgb,
    },
    StrokeRect {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: Rgb,
        stroke: f32,
    },
    Polygon {
        points: Vec<(f32, f32)>,
        color: Rgb,
    },
    Line {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        color: Rgb,
        stroke: f32,
    },
    Text {
        x: f32,
        y: f32,
        size: f32,
        bold: bool,
        color: Rgb,
        text: String,
    },
}

/// A canvas that keeps every mark, for tests and for a caller that wants to
/// replay the chart onto its own surface.
#[derive(Debug, Default, Clone)]
pub struct Record {
    pub paints: Vec<Paint>,
}

impl Canvas for Record {
    fn fill_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: Rgb) {
        self.paints.push(Paint::FillRect {
            x,
            y,
            width,
            height,
            color,
        });
    }

    fn stroke_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: Rgb, stroke: f32) {
        self.paints.push(Paint::StrokeRect {
            x,
            y,
            width,
            height,
            color,
            stroke,
        });
    }

    fn fill_polygon(&mut self, points: &[(f32, f32)], color: Rgb) {
        self.paints.push(Paint::Polygon {
            points: points.to_vec(),
            color,
        });
    }

    fn stroke_line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, color: Rgb, stroke: f32) {
        self.paints.push(Paint::Line {
            x1,
            y1,
            x2,
            y2,
            color,
            stroke,
        });
    }

    fn text(&mut self, x: f32, y: f32, size: f32, bold: bool, color: Rgb, text: &str) {
        self.paints.push(Paint::Text {
            x,
            y,
            size,
            bold,
            color,
            text: text.to_string(),
        });
    }
}
