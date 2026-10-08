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

#[tauri::command]
fn render_report(template_path: String, document_path: Option<String>) -> Result<Rendered, String> {
    let template = std::fs::read_to_string(&template_path)
        .map_err(|err| format!("could not read the template: {err}"))?;
    let document = match document_path.as_deref().filter(|path| !path.is_empty()) {
        Some(path) => {
            let text = std::fs::read_to_string(path)
                .map_err(|err| format!("could not read the document: {err}"))?;
            serde_json::from_str(&text).map_err(|err| format!("the document is not JSON: {err}"))?
        }
        None => serde_json::json!({}),
    };
    let output =
        pdfrost_engine::render(&template, &document, Utc::now()).map_err(|err| err.to_string())?;
    Ok(Rendered {
        pdf: output.pdf,
        warnings: warning_dtos(output.warnings),
    })
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
            validate_template,
            read_template,
            write_template,
            load_report,
            save_report
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
