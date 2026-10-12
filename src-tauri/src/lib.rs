use chrono::Utc;
use serde::Serialize;

#[derive(Serialize)]
struct Rendered {
    pdf: Vec<u8>,
    warnings: Vec<WarningDto>,
}

#[derive(Serialize)]
struct WarningDto {
    tag: String,
    location: String,
}

#[derive(Serialize)]
struct ValidateResult {
    ok: bool,
    error: Option<String>,
    warnings: Vec<WarningDto>,
}

fn warning_dtos(warnings: Vec<pdfrost_engine::Warning>) -> Vec<WarningDto> {
    warnings
        .into_iter()
        .map(|warning| WarningDto {
            tag: warning.tag,
            location: warning.location,
        })
        .collect()
}

fn render_loaded(
    template: &str,
    asset_dir: Option<&std::path::Path>,
    document_path: Option<String>,
) -> Result<Rendered, String> {
    let output = match document_path.as_deref().filter(|path| !path.is_empty()) {
        Some(path) => {
            let file = std::fs::File::open(path)
                .map_err(|err| format!("could not read the document: {err}"))?;
            pdfrost_engine::render_read_with_assets(template, file, Utc::now(), asset_dir)
        }
        None => pdfrost_engine::render_with_assets(
            template,
            &serde_json::json!({}),
            Utc::now(),
            asset_dir,
        ),
    }
    .map_err(|err| err.to_string())?;
    Ok(Rendered {
        pdf: output.pdf,
        warnings: warning_dtos(output.warnings),
    })
}

#[tauri::command]
fn render_report(template_path: String, document_path: Option<String>) -> Result<Rendered, String> {
    let template = std::fs::read_to_string(&template_path)
        .map_err(|err| format!("could not read the template: {err}"))?;
    render_loaded(
        &template,
        std::path::Path::new(&template_path).parent(),
        document_path,
    )
}

/// Render the report currently open in the designer, including edits that have not been saved.
#[tauri::command]
fn render_template(
    template: String,
    asset_dir: Option<String>,
    document_path: Option<String>,
) -> Result<Rendered, String> {
    let asset = asset_dir
        .as_deref()
        .filter(|path| !path.is_empty())
        .map(std::path::Path::new);
    render_loaded(&template, asset, document_path)
}

/// JRXML for the in-memory report, using the same writer as Save.
#[tauri::command]
fn report_xml(mut report: pdfrost_engine::Report) -> Result<String, String> {
    report.column_width = (report.page_width - report.left_margin - report.right_margin).max(0.0);
    let xml = pdfrost_engine::write_report(&report);
    pdfrost_engine::parse(&xml).map_err(|err| format!("the template could not be parsed: {err}"))?;
    Ok(xml)
}

#[tauri::command]
fn validate_template(contents: String) -> ValidateResult {
    match pdfrost_engine::parse(&contents) {
        Ok(report) => ValidateResult {
            ok: true,
            error: None,
            warnings: warning_dtos(report.warnings),
        },
        Err(err) => ValidateResult {
            ok: false,
            error: Some(err.to_string()),
            warnings: Vec::new(),
        },
    }
}

#[tauri::command]
fn read_template(path: String) -> Result<String, String> {
    std::fs::read_to_string(&path).map_err(|err| format!("could not read the template: {err}"))
}

#[tauri::command]
fn write_template(path: String, contents: String) -> Result<(), String> {
    std::fs::write(&path, contents).map_err(|err| format!("could not save the template: {err}"))
}

#[tauri::command]
fn read_image(template_path: String, source: String) -> Result<String, String> {
    let root = std::path::Path::new(&template_path).parent();
    let bytes = pdfrost_engine::read_image_bytes(&source, root)
        .ok_or_else(|| format!("could not read {source}"))?;
    Ok(format!("data:{};base64,{}", image_mime(&bytes), base64(&bytes)))
}

fn image_mime(bytes: &[u8]) -> &'static str {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        "image/png"
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        "image/jpeg"
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        "image/gif"
    } else if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        "image/webp"
    } else {
        "application/octet-stream"
    }
}

fn base64(bytes: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    let mut index = 0;
    while index + 3 <= bytes.len() {
        let value = ((bytes[index] as u32) << 16)
            | ((bytes[index + 1] as u32) << 8)
            | bytes[index + 2] as u32;
        out.push(TABLE[((value >> 18) & 63) as usize] as char);
        out.push(TABLE[((value >> 12) & 63) as usize] as char);
        out.push(TABLE[((value >> 6) & 63) as usize] as char);
        out.push(TABLE[(value & 63) as usize] as char);
        index += 3;
    }
    if index < bytes.len() {
        let first = bytes[index] as u32;
        let second = if index + 1 < bytes.len() {
            bytes[index + 1] as u32
        } else {
            0
        };
        let value = (first << 16) | (second << 8);
        out.push(TABLE[((value >> 18) & 63) as usize] as char);
        out.push(TABLE[((value >> 12) & 63) as usize] as char);
        if index + 1 < bytes.len() {
            out.push(TABLE[((value >> 6) & 63) as usize] as char);
            out.push('=');
        } else {
            out.push('=');
            out.push('=');
        }
    }
    out
}

#[tauri::command]
fn load_report(path: String) -> Result<pdfrost_engine::Report, String> {
    let template = std::fs::read_to_string(&path)
        .map_err(|err| format!("could not read the template: {err}"))?;
    pdfrost_engine::parse(&template).map_err(|err| err.to_string())
}

#[tauri::command]
fn save_report(path: String, mut report: pdfrost_engine::Report) -> Result<(), String> {
    report.column_width = (report.page_width - report.left_margin - report.right_margin).max(0.0);
    let xml = pdfrost_engine::write_report(&report);
    pdfrost_engine::parse(&xml).map_err(|err| format!("the template could not be parsed: {err}"))?;
    std::fs::write(&path, xml).map_err(|err| format!("could not save the template: {err}"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            render_report,
            render_template,
            report_xml,
            validate_template,
            read_template,
            write_template,
            load_report,
            save_report,
            read_image
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
