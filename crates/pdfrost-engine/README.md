# pdfrost-engine

This is the Rust library behind pdfrost. Give it a report template and one JSON object, and it gives back a PDF.

The JSON object is the data. A text field reads one value from it, such as a company name. A table or a chart can walk a list inside it and print one row, or one point, for each entry. pdfrost does not open a database connection, and one is not planned.

This library is the small piece another Rust program embeds when it needs to print a receipt, an invoice, or a statement. That program already has the page design and the details of one document as JSON. The call returns a PDF, with no window and no database. The samples are that kind of paperwork: a goods receipt, a shipment, a stock adjustment, and transfers between branches, locators, and warehouses. For a full reporting server, with a shared catalog, schedules, and database queries, use the JasperReports suite.

For the pieces it supports, the behavior follows JasperReports 6.20.1. The list is in [docs/jrxml-subset.md](../../docs/jrxml-subset.md).

## Dependency

From another project, once this repository is available at `github.com/xeonn/pdfrost`:

```toml
[dependencies]
pdfrost-engine = { git = "https://github.com/xeonn/pdfrost" }
chrono = { version = "0.4", default-features = false, features = ["clock", "std"] }
serde_json = "1"
```

From a crate inside this workspace:

```toml
pdfrost-engine = { path = "../pdfrost-engine" }
```

The library name in Rust is `pdfrost_engine`. Rust 1.92 or newer is required.

## Render

```rust
use chrono::Utc;
use serde_json::json;
use pdfrost_engine::render;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let template = std::fs::read_to_string("report.jrxml")?;
    let document = json!({
        "CompanyName": "Northwind Traders",
        "ContactName": "Maria Anders",
        "itemsList": [
            { "sku": "A-1", "qty": 2 }
        ]
    });

    let output = render(&template, &document, Utc::now())?;
    std::fs::write("report.pdf", &output.pdf)?;
    for warning in &output.warnings {
        eprintln!("{} {}", warning.tag, warning.location);
    }
    Ok(())
}
```

| Argument | Meaning |
| --- | --- |
| `template` | The text of the `.jrxml` file. |
| `document` | One JSON object. This is the data. `$F{CompanyName}` prints `Northwind Traders`. A table aimed at `itemsList` prints one row for A-1. |
| `Utc::now()` | The date and time printed when the template asks for the current time. It is shown in UTC, for example `2026-10-12 00:00:00`. Pass any `DateTime<Utc>` to use a fixed clock. |

`output.pdf` is the file. `output.warnings` names anything the template asked for that pdfrost skipped, including a missing value. `output.placements` is where each piece landed on the page, measured in points from the top-left corner.

If the call returns an error, there is no PDF. That happens when the template cannot be read, the file is not a report, or the page size is missing. A warning on its own still returns a PDF.

## Read and save a template

`parse` reads a template into a form the desktop app can edit, without drawing it. `write_report` saves that form back to a `.jrxml` file. Saving keeps the pieces pdfrost supports. Anything else in the original file is left out.

```rust
let report = pdfrost_engine::parse(template)?;
let xml = pdfrost_engine::write_report(&report);
```

## Tests

From the repository root:

```bash
cargo test -p pdfrost-engine
```
