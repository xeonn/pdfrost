/// An sRGB color. `a` is 255 when the color is opaque.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn with_alpha(self, a: u8) -> Self {
        Self { a, ..self }
    }
}

/// Chart.js leaves the canvas transparent. On a white page that is white.
pub const CHART_BACKGROUND: Rgb = Rgb::new(255, 255, 255);

/// Chart.js `defaults.color` (`#666`). Title, ticks, and legend text.
pub const TEXT: Rgb = Rgb::new(0x66, 0x66, 0x66);

/// `defaults.borderColor`, `rgba(0, 0, 0, 0.1)`, on white. Gridlines and the axis frame.
pub const GRID: Rgb = Rgb::new(230, 230, 230);

/// Chart.js 4.4.6 colors plugin `BORDER_COLORS`. The list repeats after gray.
const SERIES: &[Rgb] = &[
    Rgb::new(54, 162, 235),
    Rgb::new(255, 99, 132),
    Rgb::new(255, 159, 64),
    Rgb::new(255, 205, 86),
    Rgb::new(75, 192, 192),
    Rgb::new(153, 102, 255),
    Rgb::new(201, 203, 207),
];

/// The colors plugin fills bars and areas with the border color at 50% alpha.
const FILL_ALPHA: u8 = 128;

pub fn series_color(index: usize) -> Rgb {
    SERIES[index % SERIES.len()]
}

pub fn series_fill(index: usize) -> Rgb {
    series_color(index).with_alpha(FILL_ALPHA)
}
