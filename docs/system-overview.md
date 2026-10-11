# System overview

pdfrost generates a PDF from a `.jrxml` report template and shows that PDF in a desktop app. Another Rust program can do the same generation without the app.

## Problem

The templates in `jrxmls/` are JasperReports designs: warehouse and sample documents with a page, labels, field values, rules, boxes, and a line-item table. pdfrost needs to turn one of those files, plus the JSON document that fills it, into a PDF a person can read.

## Subset of JasperReports

[JasperReports](https://github.com/Jaspersoft/jasperreports) is the reference project. It is written in Java. It loads a JRXML design, fills it, and exports to several formats, including HTML, PDF, Excel, and Word.

pdfrost implements a subset of that library, in Rust. The subset is PDF generation for the tags and expressions in [JRXML subset](jrxml-subset.md). It is not a port of the Java project. HTML, Excel, Word, and the rest of the JasperReports surface stay outside the subset. A `subreport` is in the subset when its expression is a `.jrxml` path and its data is the current JSON object or a `subDataSource` array. An `image` element is in the subset when its expression is a local file path or a field that yields one. Category charts (`areaChart`, `barChart`, `lineChart`, and the stacked forms) are in the subset. The plot is [pdfrost-charts](charts.md), drawn in the Chart.js 4.4.6 colors with the page font. Other chart tags stay a placeholder and a warning.

For a feature inside the subset, JasperReports 6.20.1 decides the behavior. Page geometry, band order, element position, expression results, and the marks on the page should match the PDF that library writes from the same template and the same data. Two differences are intentional: a missing value prints blank instead of the word `null`, and `new java.util.Date()` prints `YYYY-MM-DD HH:MM:SS` in UTC instead of a locale-specific date. Fill and layout rules the samples rely on are listed in [JRXML subset](jrxml-subset.md#behaviors-the-samples-require).

## Reference project

The templates that record a version were saved with Jaspersoft Studio 6.20.1.final against JasperReports Library `6.20.1`. That release is the tag to read. `master` and the 7.x line reorganized the Java library, so they are the wrong guide for these files.

The Java source is not cloned into this repository. pdfrost does not compile or link against it. Consult the `6.20.1` tag on GitHub, or a checkout kept in another directory, when a supported construct is ambiguous.

## Outputs

Four outputs share one engine. Parsing and PDF generation stay in Rust.

| Output | Role | When |
| --- | --- | --- |
| Core engine | A Rust library. Template and data in, PDF bytes out. No window. | First |
| Go package | A Go module that calls that same engine, so a Go program can render without reimplementing the subset. | Once the engine renders the samples |
| Viewer | A Tauri desktop app that opens a template, calls the engine, and displays the PDF. | With the engine |
| Designer | The same Tauri app, extended to create and save a `.jrxml` file and preview it through the engine. | After the viewer is useful |

The engine is what other Rust programs embed. The Go package is what other Go programs import. The desktop app is a third caller. The contract for the Go side is in [Go package](go-package.md).

## Acceptance set

`jrxmls/` is the set of documents the subset must learn to print:

- `Sample.jrxml` covers a title, column headings, text fields, a line, a rectangle, and one of each supported category chart. The charts plot `Units` and `Returns` from `chartPoints`.
- `Subreport.jrxml` fills company and contact from `SubreportData.json` and repeats `OrderLines.jrxml` over `orders`. Each order prints its number and a table of `lines`.
- `Receive.jrxml`, `ShipmentOut.jrxml`, `StockAdjustment.jrxml`, `InterBranchTransfer.jrxml`, `InterLocatorTransfer.jrxml`, and `InterWarehouseTransfer.jrxml` are the operational forms: labels, fields, rules, boxes, a table fed from `itemsList`, a page number, and a printed-at time.

A finished slice of the engine is one that renders the templates it claims, with a fixture, to a PDF that can be compared with JasperReports 6.20.1.

## What this system does not do

- Compile JRXML to a `.jasper` file, or compile expressions to Java bytecode.
- Run a SQL or JSON query. The caller passes one JSON document.
- Open a database, a network service, or a window from the engine.
- Evaluate general Java expressions, parameters, or resource-bundle keys.
- Export any format other than PDF, or set PDF encryption, PDF/A, metadata, or bookmarks.
- Stretch text out of its element box, or print "Page X of Y".
- Vendor or build the JasperReports source tree.
