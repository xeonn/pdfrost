# Milestones

Each milestone leaves a program that does something new. Later milestones sit on earlier ones. The check is a file in `jrxmls/`, rendered with a fixture and compared with the PDF JasperReports 6.20.1 writes from the same template and data when the layout is unclear.

The tag rules for every milestone are in [JRXML subset](jrxml-subset.md). The engine crate is created before milestone 1.

## 1. Report model

Parse a `.jrxml` file into typed Rust data: name, page size, margins, styles, fields (including `class` and the JSON path property), bands (`splitType`, height), and elements in document order. Keep a `subDataset`'s fields on that dataset. Keep pen width, color, and line style, paragraph indents, vertical alignment, and the `style` name on a table section. Static text, text fields, lines, and rectangles come first. Table cell contents may wait until milestone 4. `whenNoDataType`, column widths, and section heights are kept now. Unsupported tags, including the area chart, are warnings with a location.

**Done when** each file in `jrxmls/` loads, and a test can read the page size, a band's `splitType`, a pen, a text element, and a subdataset field from the model.

## 2. Static PDF

Draw one page from elements that do not need data: static text, lines, and rectangles, using the template page size and margins. Embed the default sans. Clip text to the element box. Draw solid pens and alignment.

**Done when** `Sample.jrxml` writes a PDF whose title and column headings sit where the template places them, in the embedded font. The area chart is a placeholder box and a warning, not a plot.

## 3. Fill fields

Accept one JSON object on the render call and treat it as the only main record. Evaluate `$F{…}`. Print integer fields as digits. A missing value is blank plus a warning, not the word `null`. The engine does not run `queryString`.

**Done when** a fixture fills the company and contact columns of `Sample.jrxml`, and a header field such as `docno` on one operational template. A missing field does not appear as the text `null`.

## 4. Pages, tables, and repeating rows

Repeat table detail rows from `subDataSource("itemsList")`. Honor `splitType="Stretch"`. Keep page headers and footers on every page. Continue rows on the next page when the band is full. An empty `itemsList` on a table with `AllSectionsNoDetail` still draws headers. An empty column footer still takes its height. Evaluate `$V{PAGE_NUMBER}` and print `new java.util.Date()` as `YYYY-MM-DD HH:MM:SS` UTC.

**Done when** a multi-row fixture for one operational template produces a PDF with a line-item table, a page number, and that timestamp, and a second page when the rows do not fit. Page 2 still shows the table column headers. `Receive.jrxml` with an empty `itemsList` still shows the table header.

## 5. Every sample through the crate

The library crate already exists. This milestone runs every template in `jrxmls/` through `render` with its fixture. Style names, integer quantities, and nested JSON paths that the earlier milestones did not hit get closed here. Charts, stretch-with-overflow, and "Page X of Y" stay warnings.

**Done when** each template and fixture pair renders to PDF, and each operational PDF's text contains its document number and one line-item field. No desktop app is required. `Sample.jrxml` plots its five category charts from `chartPoints`.

## Go package

After milestone 5, the Go package embeds `engine.wasm` and renders the same fixture the Rust tests use.

**Done when** `go test ./...` in `go/` returns PDF bytes for one sample template. The test runs the committed `engine.wasm`. The Go package does not contain a second implementation of the subset. `Sample.jrxml` plots its category charts from `chartPoints`, so the result has no `chart` warning.

## 6. Viewer

Open a `.jrxml` file and a JSON file from the desktop window, render through the engine, and display the PDF and the warnings.

**Done when** `Sample.jrxml` and one operational template can be opened in the app and read as a PDF there.

## 7. Visual review

After the viewer can show a document, check positions against the JRXML numbers: label origin, column width, and pen width. Read the same pages in the window. A JasperReports 6.20.1 PDF, produced outside this repo, settles a rule the template does not state in numbers. It is not a test fixture.

**Done when** those position checks pass for every operational template. `Sample.jrxml` places its five category charts in the summary and plots them there.

## 8. Designer

After the visual review, extend the window so a template can be created and saved as `.jrxml`. The writer for the supported tags already exists. The screen plan is in [Designer](designer.md). Preview still chooses a JSON document and uses the milestone 6 path.

**Done when** a template built in the app reloads through the parser and previews without hand-written XML.
