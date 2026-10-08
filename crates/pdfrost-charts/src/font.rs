use skrifa::instance::{LocationRef, Size};
use skrifa::raw::TableProvider;
use skrifa::{FontRef, MetadataProvider};

/// Width and ascent for the face the chart is allowed to draw with.
pub trait Fonts {
    fn width(&self, text: &str, size: f32, bold: bool) -> f32;
    fn ascent(&self, size: f32, bold: bool) -> f32;
}

/// A regular face and a bold face, measured with skrifa.
///
/// Pass the same bytes the PDF embeds. The chart does not choose a family.
pub struct SkrifaFace<'a> {
    pub regular: &'a [u8],
    pub bold: &'a [u8],
}

impl Fonts for SkrifaFace<'_> {
    fn width(&self, text: &str, size: f32, bold: bool) -> f32 {
        measure(self.bytes(bold), text, size)
    }

    fn ascent(&self, size: f32, bold: bool) -> f32 {
        ascent(self.bytes(bold), size)
    }
}

impl SkrifaFace<'_> {
    fn bytes(&self, bold: bool) -> &[u8] {
        if bold {
            self.bold
        } else {
            self.regular
        }
    }
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

fn ascent(bytes: &[u8], size: f32) -> f32 {
    let Ok(font) = FontRef::from_index(bytes, 0) else {
        return size * 0.8;
    };
    let units_per_em = font.head().map(|head| head.units_per_em()).unwrap_or(1000);
    let ascender = font
        .hhea()
        .map(|hhea| hhea.ascender().to_i16())
        .unwrap_or(800);
    f32::from(ascender) * size / f32::from(units_per_em)
}
