# pdfrost-engine

Rust library that reads a JRXML template, fills it from one JSON document, and returns PDF bytes. JasperReports 6.20.1 is the behavior reference for the supported tags. The tag list is in [docs/jrxml-subset.md](../../docs/jrxml-subset.md).

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

The library target is `pdfrost_engine`. Rust 1.92 or newer is required.

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
| `template` | The `.jrxml` text. |
| `document` | One JSON object. `$F{name}` reads a property. A table `subDataSource("itemsList")` repeats over the array `itemsList`. |
| `Utc::now()` | The instant printed for `new java.util.Date()`, as `YYYY-MM-DD HH:MM:SS` in UTC. Pass any `DateTime<Utc>` to fix that clock. |

`output.pdf` is the file bytes. `output.warnings` names unsupported tags, unsupported expressions, and missing fields. `output.placements` is one box per drawn element, in PDF points from the top-left of the page.

`Err` means there is no PDF. That happens when the XML cannot be read, the root is not `jasperReport`, or the page size is not positive. A warning by itself is still `Ok`.

## Parse and write

`parse` turns JRXML into a `Report` without drawing it. `write_report` writes that report back to JRXML for the supported subset. Round-tripping drops tags the subset does not store.

```rust
let report = pdfrost_engine::parse(template)?;
let xml = pdfrost_engine::write_report(&report);
```

## Tests

From the repository root:

```bash
cargo test -p pdfrost-engine
```
