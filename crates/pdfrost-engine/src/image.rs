//! Load and fit an `<image>` into its element box.
//!
//! A string expression is a file path, tried beside the template and then as
//! given, or the picture itself: raw base64 of a PNG or JPEG, or a
//! `data:image/...;base64,` URI. `$F{name}` uses a `pdfrost.` property of that
//! name when the image has one, then the field text. A Java subset reads
//! `new java.io.File(...)`, `new java.net.URL(...)`, and a
//! `ByteArrayInputStream` around a Base64 decode of a literal or `$F{name}`.
//! A missing picture follows `onErrorType`.

use std::path::Path;

use base64::Engine as _;

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
    let source = match image_source(image, fields, data, root, warnings) {
        Some(source) => source,
        None => return missing(image),
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

fn image_source(
    image: &Image,
    fields: &[crate::model::Field],
    data: &Value,
    root: &Value,
    warnings: &mut Vec<Warning>,
) -> Option<String> {
    if let Some(argument) = studio_argument(&image.expression) {
        return argument_text(image, argument, fields, data, root, warnings);
    }
    match Expression::parse(&image.expression) {
        Expression::Empty => None,
        Expression::Text(text) => Some(text),
        Expression::Field(name) => argument_text(
            image,
            StudioArgument::Field(name),
            fields,
            data,
            root,
            warnings,
        ),
        Expression::Unknown(text) => {
            warnings.push(Warning {
                tag: text,
                location: "image".to_string(),
            });
            None
        }
        _ => {
            warnings.push(Warning {
                tag: image.expression.clone(),
                location: "image".to_string(),
            });
            None
        }
    }
}

#[derive(Debug)]
enum StudioArgument {
    Text(String),
    Field(String),
}

fn argument_text(
    image: &Image,
    argument: StudioArgument,
    fields: &[crate::model::Field],
    data: &Value,
    root: &Value,
    warnings: &mut Vec<Warning>,
) -> Option<String> {
    match argument {
        StudioArgument::Text(text) => Some(text),
        StudioArgument::Field(name) => {
            if let Some(picture) = image.pictures.get(&name) {
                let picture = picture.trim();
                if !picture.is_empty() {
                    return Some(picture.to_string());
                }
            }
            let filled = fill::field(&name, fields, data, root, "image");
            if let Some(warning) = filled.warning {
                warnings.push(warning);
            }
            if filled.text.is_empty() {
                None
            } else {
                Some(filled.text)
            }
        }
    }
}

/// Studio image constructors this engine evaluates. Anything else stays unknown.
fn studio_argument(expression: &str) -> Option<StudioArgument> {
    let compact = compact_outside_strings(expression);
    let body = strip_null_ternary(&compact);
    let body = strip_byte_stream(body);
    for prefix in [
        "new java.io.File(",
        "new java.net.URL(",
        "java.util.Base64.getMimeDecoder().decode(",
        "java.util.Base64.getDecoder().decode(",
        "org.apache.commons.codec.binary.Base64.decodeBase64(",
        "net.sf.jasperreports.util.Base64Util.decode(",
    ] {
        if let Some(argument) = call_argument(body, prefix) {
            return Some(argument);
        }
    }
    None
}

fn strip_null_ternary(body: &str) -> &str {
    match body.find("?null:") {
        Some(index) => &body[index + "?null:".len()..],
        None => body,
    }
}

fn strip_byte_stream(body: &str) -> &str {
    const PREFIX: &str = "new java.io.ByteArrayInputStream(";
    let Some(rest) = body.strip_prefix(PREFIX) else {
        return body;
    };
    let Some(inner) = rest.strip_suffix(')') else {
        return body;
    };
    if paren_balanced(inner) { inner } else { body }
}

fn paren_balanced(source: &str) -> bool {
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escape = false;
    for ch in source.chars() {
        if in_string {
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth < 0 {
                    return false;
                }
            }
            _ => {}
        }
    }
    depth == 0 && !in_string
}

fn call_argument(body: &str, prefix: &str) -> Option<StudioArgument> {
    let inside = body.strip_prefix(prefix)?.strip_suffix(')')?;
    if !paren_balanced(inside) {
        return None;
    }
    studio_value(inside)
}

fn studio_value(inside: &str) -> Option<StudioArgument> {
    if let Some(name) = inside.strip_prefix("$F{") {
        let (name, suffix) = name.split_once('}')?;
        if name.is_empty() || !field_suffix(suffix) {
            return None;
        }
        return Some(StudioArgument::Field(name.to_string()));
    }
    let text = inside.strip_prefix('"')?.strip_suffix('"')?;
    if text.contains('\n') {
        return None;
    }
    Some(StudioArgument::Text(text.replace("\\\"", "\"").replace("\\\\", "\\")))
}

fn field_suffix(suffix: &str) -> bool {
    matches!(
        suffix,
        "" | ".trim()" | ".getBytes()" | ".trim().getBytes()"
    )
}

fn compact_outside_strings(input: &str) -> String {
    let mut out = String::new();
    let mut in_string = false;
    let mut escape = false;
    let mut pending_space = false;
    for ch in input.chars() {
        if in_string {
            out.push(ch);
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        if ch == '"' {
            pending_space = false;
            in_string = true;
            out.push(ch);
            continue;
        }
        if ch.is_whitespace() {
            pending_space = !out.is_empty();
            continue;
        }
        let word = out
            .chars()
            .last()
            .is_some_and(|prev| prev.is_ascii_alphanumeric())
            && ch.is_ascii_alphanumeric();
        if pending_space && word {
            out.push(' ');
        }
        pending_space = false;
        out.push(ch);
    }
    out
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

/// Read a picture. `asset_dir` is the template's directory.
///
/// A `data:` URI and raw base64 are decoded here. Anything else is a file path,
/// tried beside the template and then as given. A path that is not a picture is
/// then tried as base64, so a field can carry the bytes when there is no file.
pub fn read_image_bytes(source: &str, asset_dir: Option<&Path>) -> Option<Vec<u8>> {
    let source = source.trim();
    if source.is_empty() || source.contains('\0') {
        return None;
    }
    if source.starts_with("data:") {
        return decode_inline_image(source);
    }
    if is_remote(source) {
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
    decode_inline_image(source)
}

/// PNG or JPEG carried in the expression: a data URI, or raw base64.
fn decode_inline_image(source: &str) -> Option<Vec<u8>> {
    let payload = if let Some(rest) = source.trim().strip_prefix("data:") {
        let (_, data) = rest.split_once(',')?;
        data
    } else {
        source
    };
    let compact: String = payload.chars().filter(|ch| !ch.is_whitespace()).collect();
    if compact.len() < 16 {
        return None;
    }
    if !compact
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/' | b'='))
    {
        return None;
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(compact.as_bytes())
        .ok()?;
    image_pixel_size(&bytes)?;
    Some(bytes)
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
    fn studio_constructors_reduce_to_a_path_or_a_field() {
        assert!(matches!(
            studio_argument(r#"new java.io.File("logo.png")"#),
            Some(StudioArgument::Text(text)) if text == "logo.png"
        ));
        assert!(matches!(
            studio_argument("new java.net.URL($F{site})"),
            Some(StudioArgument::Field(name)) if name == "site"
        ));
        let java = r#"
            $F{logoBase64} == null || $F{logoBase64}.trim().isEmpty()
              ? null
              : new java.io.ByteArrayInputStream(
                  java.util.Base64.getMimeDecoder().decode($F{logoBase64}.trim()))
        "#;
        assert!(matches!(
            studio_argument(java),
            Some(StudioArgument::Field(name)) if name == "logoBase64"
        ));
        assert!(studio_argument("new java.awt.Image($F{logo})").is_none());
    }

    #[test]
    fn raw_base64_and_a_data_uri_are_the_picture() {
        const png: &[u8] = &[
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00,
            0x00, 0x90, 0x77, 0x53, 0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x78,
            0x9C, 0x63, 0xF8, 0xCF, 0xC0, 0x00, 0x00, 0x03, 0x01, 0x01, 0x00, 0xC9, 0xFE, 0x92,
            0xEF, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];
        let encoded = base64::engine::general_purpose::STANDARD.encode(png);
        assert_eq!(read_image_bytes(&encoded, None).as_deref(), Some(png));
        let uri = format!("data:image/png;base64,{encoded}");
        assert_eq!(read_image_bytes(&uri, None).as_deref(), Some(png));
        assert!(read_image_bytes("logo.png", None).is_none());
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
