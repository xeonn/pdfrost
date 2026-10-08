use krilla::page::PageSettings;
use krilla::Document;

#[test]
fn krilla_writes_a_blank_page() {
    let mut document = Document::new();
    let page = document.start_page_with(PageSettings::from_wh(200.0, 200.0).unwrap());
    page.finish();
    let pdf = document.finish().expect("blank page");
    assert!(pdf.starts_with(b"%PDF"));
    let text = pdf_extract::extract_text_from_mem(&pdf).expect("text extract");
    assert!(text.trim().is_empty());
}
