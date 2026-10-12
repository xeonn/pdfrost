//! Load and fit an `<image>` into its element box.
//!
//! A string expression is a file path, tried beside the template and then as
//! given. `$F{name}` uses the field text the same way. A missing picture
//! follows `onErrorType`.

use std::path::Path;

use serde_json::Value;

use crate::expr::Expression;
use crate::fill;
use crate::model::{HorizontalImageAlign, Image, OnErrorType, ScaleImage, VerticalAlignment};
use crate::Warning;

pub(crate) struct Bitmap {
    pub bytes: Vec<u8>,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub extra_height: f32,
    pub rotation: Option<crate::model::Rotation>,
}

pub(crate) enum Prepared {
    Bitmap(Bitmap),
    Icon,
    Blank,
}

impl Prepared {
    pub(crate) fn extra_height(&self) -> f32 {
        match self {
            Prepared::Bitmap(bitmap) => bitmap.extra_height,
            Prepared::Icon | Prepared::Blank => 0.0,
        }
    }
}

pub(crate) fn prepare(
    image: &Image,
    fields: &[crate::model::Field],
    data: &Value,
    root: &Value,
    asset_dir: Option<&Path>,
    warnings: &mut Vec<Warning>,
) -> Prepared {
    let source = match Expression::parse(&image.expression) {
        Expression::Empty => return missing(image),
        Expression::Text(text) => text,
        Expression::Field(name) => {
            let filled = fill::field(&name, fields, data, root, "image");
            if let Some(warning) = filled.warning {
                warnings.push(warning);
            }
            if filled.text.is_empty() {
                return missing(image);
            }
            filled.text
        }
        Expression::Unknown(text) => {
            warnings.push(Warning {
                tag: text,
                location: "image".to_string(),
            });
            return missing(image);
        }
        _ => {
            warnings.push(Warning {
                tag: image.expression.clone(),
                location: "image".to_string(),
            });
            return missing(image);
        }
    };

    let Some(bytes) = read_image_bytes(&source, asset_dir) else {
        let remote = is_remote(&source);
        if on_error(image) == OnErrorType::Error {
            warnings.push(Warning {
                tag: if remote {
                    source
                } else {
                    format!("image:{source}")
                },
                location: "image".to_string(),
            });
        }
        return missing(image);
    };
    let Some((pixel_w, pixel_h)) = image_pixel_size(&bytes) else {
        if on_error(image) == OnErrorType::Error {
            warnings.push(Warning {
                tag: format!("image:{source}"),
                location: "image".to_string(),
            });
        }
        return missing(image);
    };
    let fitted = fit_image(
        image.scale_image.unwrap_or(ScaleImage::RetainShape),
        image.h_align.unwrap_or(HorizontalImageAlign::Left),
        image.v_align.unwrap_or(VerticalAlignment::Top),
        image.element.width,
        image.element.height,
        pixel_w,
        pixel_h,
    );
    Prepared::Bitmap(Bitmap {
        bytes,
        x: fitted.x,
        y: fitted.y,
        width: fitted.width,
        height: fitted.height,
        extra_height: fitted.extra_height,
        rotation: image.rotation,
    })
}

fn missing(image: &Image) -> Prepared {
    match on_error(image) {
        OnErrorType::Icon => Prepared::Icon,
        OnErrorType::Error | OnErrorType::Blank => Prepared::Blank,
    }
}

fn on_error(image: &Image) -> OnErrorType {
    image.on_error_type.unwrap_or(OnErrorType::Error)
}

fn is_remote(source: &str) -> bool {
    let source = source.trim();
    source.starts_with("http://") || source.starts_with("https://")
}

/// Read a local picture. `asset_dir` is the template's directory.
pub fn read_image_bytes(source: &str, asset_dir: Option<&Path>) -> Option<Vec<u8>> {
    let source = source.trim();
    if source.is_empty() || is_remote(source) || source.contains('\0') {
        return None;
    }
    let path = Path::new(source);
    let mut candidates = Vec::new();
    if path.is_absolute() {
        candidates.push(path.to_path_buf());
    } else if let Some(dir) = asset_dir {
        candidates.push(dir.join(path));
    }
    if !path.is_absolute() {
        candidates.push(path.to_path_buf());
    }
    for candidate in candidates {
        if let Ok(bytes) = std::fs::read(&candidate) {
            if image_pixel_size(&bytes).is_some() {
                return Some(bytes);
            }
        }
    }
    None
}

pub(crate) fn krilla_image(bytes: &[u8]) -> Option<krilla::image::Image> {
    let decoded = image::load_from_memory(bytes).ok()?;
    let rgba = decoded.to_rgba8();
    let (width, height) = rgba.dimensions();
    if width == 0 || height == 0 {
        return None;
    }
    let raw = rgba.into_raw();
    if raw.len() != width as usize * height as usize * 4 {
        return None;
    }
    Some(krilla::image::Image::from_rgba8(raw, width, height))
}

pub(crate) fn image_pixel_size(bytes: &[u8]) -> Option<(f32, f32)> {
    let image = krilla_image(bytes)?;
    let (width, height) = image.size();
    if width == 0 || height == 0 {
        None
    } else {
        Some((width as f32, height as f32))
    }
}

pub(crate) struct Fitted {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub extra_height: f32,
}

pub(crate) fn fit_image(
    scale: ScaleImage,
    h_align: HorizontalImageAlign,
    v_align: VerticalAlignment,
    box_w: f32,
    box_h: f32,
    image_w: f32,
    image_h: f32,
) -> Fitted {
    if image_w <= 0.0 || image_h <= 0.0 || box_w <= 0.0 || box_h <= 0.0 {
        return Fitted {
            x: 0.0,
            y: 0.0,
            width: box_w.max(0.0),
            height: box_h.max(0.0),
            extra_height: 0.0,
        };
    }
    let (width, height, extra_height) = match scale {
        ScaleImage::Clip => (image_w, image_h, 0.0),
        ScaleImage::FillFrame => (box_w, box_h, 0.0),
        ScaleImage::RetainShape => {
            let factor = (box_w / image_w).min(box_h / image_h);
            (image_w * factor, image_h * factor, 0.0)
        }
        ScaleImage::RealHeight | ScaleImage::RealSize => {
            let factor = if image_w > box_w {
                box_w / image_w
            } else {
                1.0
            };
            let height = image_h * factor;
            (image_w * factor, height, (height - box_h).max(0.0))
        }
    };
    let frame_h = box_h + extra_height;
    Fitted {
        x: align_h(h_align, box_w, width),
        y: align_v(v_align, frame_h, height),
        width,
        height,
        extra_height,
    }
}

/// Screen rotation around the element center. `Left` is 90 degrees counter-clockwise.
pub(crate) fn rotation_transform(
    rotation: crate::model::Rotation,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) -> krilla::geom::Transform {
    let angle = match rotation {
        crate::model::Rotation::Right => 90.0,
        crate::model::Rotation::Left => -90.0,
        crate::model::Rotation::UpsideDown => 180.0,
    };
    krilla::geom::Transform::from_rotate_at(angle, x + width / 2.0, y + height / 2.0)
}

fn align_h(align: HorizontalImageAlign, frame: f32, content: f32) -> f32 {
    match align {
        HorizontalImageAlign::Left => 0.0,
        HorizontalImageAlign::Center => (frame - content) / 2.0,
        HorizontalImageAlign::Right => frame - content,
    }
}

fn align_v(align: VerticalAlignment, frame: f32, content: f32) -> f32 {
    match align {
        VerticalAlignment::Top => 0.0,
        VerticalAlignment::Middle => (frame - content) / 2.0,
        VerticalAlignment::Bottom => frame - content,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retain_shape_fits_inside_and_centers() {
        let fitted = fit_image(
            ScaleImage::RetainShape,
            HorizontalImageAlign::Center,
            VerticalAlignment::Middle,
            100.0,
            50.0,
            100.0,
            100.0,
        );
        assert_eq!(fitted.width, 50.0);
        assert_eq!(fitted.height, 50.0);
        assert_eq!(fitted.x, 25.0);
        assert_eq!(fitted.y, 0.0);
        assert_eq!(fitted.extra_height, 0.0);
    }

    #[test]
    fn real_height_grows_when_the_picture_is_taller() {
        let fitted = fit_image(
            ScaleImage::RealHeight,
            HorizontalImageAlign::Left,
            VerticalAlignment::Top,
            40.0,
            10.0,
            80.0,
            40.0,
        );
        assert_eq!(fitted.width, 40.0);
        assert_eq!(fitted.height, 20.0);
        assert_eq!(fitted.extra_height, 10.0);
    }

    #[test]
    fn rotation_keeps_the_center_fixed() {
        let transform = rotation_transform(crate::model::Rotation::Left, 10.0, 20.0, 40.0, 30.0);
        let x = transform.sx() * 30.0 + transform.kx() * 35.0 + transform.tx();
        let y = transform.ky() * 30.0 + transform.sy() * 35.0 + transform.ty();
        assert!((x - 30.0).abs() < 0.01, "{x}");
        assert!((y - 35.0).abs() < 0.01, "{y}");
    }
}
