//! WebAssembly exports for the Go package. The host copies JRXML and JSON
//! into linear memory, calls [`render`], and reads a length-prefixed blob back.

use std::alloc::{alloc, dealloc, Layout};
use std::slice;

use chrono::{DateTime, Utc};
use pdfrost_engine::Warning;
use serde_json::Value;

/// Allocates `size` bytes in the module. The host writes inputs here.
///
/// # Safety
/// The pointer is owned by the caller until [`dealloc_bytes`].
#[no_mangle]
pub extern "C" fn alloc_bytes(size: u32) -> *mut u8 {
    if size == 0 {
        return std::ptr::null_mut();
    }
    let layout = Layout::from_size_align(size as usize, 1).expect("size");
    unsafe { alloc(layout) }
}

/// # Safety
/// `ptr` must come from [`alloc_bytes`] or [`render`] and `size` must be the
/// same length that was allocated.
#[no_mangle]
pub unsafe extern "C" fn dealloc_bytes(ptr: *mut u8, size: u32) {
    if ptr.is_null() || size == 0 {
        return;
    }
    let layout = Layout::from_size_align(size as usize, 1).expect("size");
    dealloc(ptr, layout);
}

/// Renders a template. Returns `(length << 32) | pointer` of a blob the host
/// must release with [`dealloc_bytes`].
///
/// A non-zero first `u32` is an error string. A zero first `u32` is success:
/// PDF bytes, then warning strings. Each string is a little-endian length
/// followed by UTF-8 bytes.
///
/// # Safety
/// `template` and `json` must be readable for their lengths when those lengths
/// are non-zero.
#[no_mangle]
pub unsafe extern "C" fn render(
    template: *const u8,
    template_len: u32,
    json: *const u8,
    json_len: u32,
    rendered_at_unix: i64,
) -> u64 {
    let blob = match render_blob(template, template_len, json, json_len, rendered_at_unix) {
        Ok(bytes) => bytes,
        Err(message) => error_blob(&message),
    };
    leak(blob)
}

unsafe fn render_blob(
    template: *const u8,
    template_len: u32,
    json: *const u8,
    json_len: u32,
    rendered_at_unix: i64,
) -> Result<Vec<u8>, String> {
    let template = bytes_to_str(template, template_len)?;
    let document = bytes_to_str(json, json_len)?;
    let document: Value = serde_json::from_str(document).map_err(|err| err.to_string())?;
    let rendered_at = DateTime::<Utc>::from_timestamp(rendered_at_unix, 0).unwrap_or_else(Utc::now);
    match pdfrost_engine::render(template, &document, rendered_at) {
        Ok(output) => Ok(success_blob(&output.pdf, &output.warnings)),
        Err(err) => Err(err.to_string()),
    }
}

unsafe fn bytes_to_str<'a>(ptr: *const u8, len: u32) -> Result<&'a str, String> {
    if ptr.is_null() || len == 0 {
        return Ok("");
    }
    let bytes = slice::from_raw_parts(ptr, len as usize);
    std::str::from_utf8(bytes).map_err(|_| "input is not utf-8".to_string())
}

fn success_blob(pdf: &[u8], warnings: &[Warning]) -> Vec<u8> {
    let mut out = Vec::new();
    push_u32(&mut out, 0);
    push_bytes(&mut out, pdf);
    push_u32(&mut out, warnings.len() as u32);
    for warning in warnings {
        push_bytes(
            &mut out,
            format!("{} {}", warning.tag, warning.location).as_bytes(),
        );
    }
    out
}

fn error_blob(message: &str) -> Vec<u8> {
    let mut out = Vec::new();
    push_bytes(&mut out, message.as_bytes());
    out
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    push_u32(out, bytes.len() as u32);
    out.extend_from_slice(bytes);
}

fn leak(bytes: Vec<u8>) -> u64 {
    let len = bytes.len() as u32;
    if len == 0 {
        return 0;
    }
    let layout = Layout::from_size_align(bytes.len(), 1).expect("size");
    let ptr = unsafe { alloc(layout) };
    if ptr.is_null() {
        return 0;
    }
    unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len()) };
    (u64::from(len) << 32) | (ptr as u64)
}
