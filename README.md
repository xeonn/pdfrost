# pdfrost ❄️

> A high-performance, lightweight rendering engine that reads **JRXML** templates and **JSON** data to generate PDFs. Designed as a streamlined, code-friendly subset of the Jaspersoft engine and studio specifically optimized for PDF output.

---

## 🎯 What is pdfrost?

Working with full-scale enterprise reporting servers can be heavy and complex when you just want to generate a PDF from a template and structured data. **pdfrost** provides a modern, modular alternative:
* **Familiar Templates:** Use standard JRXML files (Jaspersoft format).
* **Simple Data Payload:** Feed your templates directly using standard JSON data files.
* **Laser-Focused:** Implements a clean subset of the Jaspersoft engine dedicated entirely to fast, accurate PDF generation.

---

## 🏗️ Project Components

The `pdfrost` ecosystem consists of three main components:

1. **`pdfrost-engine` (Rust Crate):** The core rendering engine written in Rust for lightning-fast performance. Perfect for native Rust applications. The library name in Rust is `pdfrost_engine`.
2. **`pdfrost` (Tauri App):** A desktop visual editor built with Tauri, allowing you to easily design and edit your JRXML templates.
3. **`github.com/xeonn/pdfrost/go` (Go Package):** A Go wrapper that integrates the core engine compiled to WebAssembly (WASM), bringing native PDF generation to Go projects.

---

## 🚀 Getting Started with Components

### 1. Rust Crate (`pdfrost-engine`)
Add `pdfrost-engine` to your `Cargo.toml` dependencies:

```toml
[dependencies]
pdfrost-engine = { git = "https://github.com/xeonn/pdfrost" }
chrono = { version = "0.4", default-features = false, features = ["clock", "std"] }
serde_json = "1"
```

Basic usage in Rust:
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
    println!("PDF generated successfully! ❄️");
    Ok(())
}
```

`template` is the text of the `.jrxml` file. `document` is the JSON data: `$F{CompanyName}` prints `Northwind Traders`, and a table aimed at `itemsList` prints one row for A-1. `Utc::now()` is the date and time printed when the template asks for the current time, shown in UTC.

### 2. pdfrost (Tauri Desktop App)
A lightweight desktop application designed for creating and tweaking your JRXML templates visually. 
*(Check the [Releases](https://github.com/xeonn/pdfrost/releases) page for pre-built installers or build from source via Tauri).*

### 3. Go Package (`github.com/xeonn/pdfrost/go`)
Use the rendering engine in your Go applications through WASM:

```bash
go get github.com/xeonn/pdfrost/go@v0.3.0
```

```go
package main

import (
    "fmt"
    "os"
    "time"

    "github.com/xeonn/pdfrost/go"
)

func main() {
    template, err := os.ReadFile("report.jrxml")
    if err != nil {
        panic(err)
    }
    document := []byte(`{
        "CompanyName": "Northwind Traders",
        "ContactName": "Maria Anders",
        "itemsList": [{"sku": "A-1", "qty": 2}]
    }`)
    result, err := pdfrost.Render(string(template), document, time.Now().UTC())
    if err != nil {
        panic(err)
    }
    if err := os.WriteFile("report.pdf", result.PDF, 0o644); err != nil {
        panic(err)
    }
    for _, warning := range result.Warnings {
        fmt.Println(warning.Message)
    }
}
```

`template` is the text of the `.jrxml` file. `document` is one JSON value: `$F{CompanyName}` prints `Northwind Traders`, and a table aimed at `itemsList` prints one row for A-1. `time.Now().UTC()` is the date and time printed when the template asks for the current time, shown in UTC. `result.PDF` is the file. `result.Warnings` lists anything the template asked for that was skipped.

---

## 📋 Supported Features (Jaspersoft Subset)

`pdfrost` focuses exclusively on PDF generation and supports a streamlined subset of JRXML elements:
* Standard report bands (Title, Detail, Summary, Page Header/Footer)
* Text fields, static text, and basic layout shapes
* JSON data-source mapping and expressions
* Basic styling, alignments, and custom fonts

---

## 🛠️ Requirements & Development

If you are looking to build or contribute from source:
* **Core Engine:** [Rust toolchain](https://www.rust-lang.org/) (1.92 or newer)
* **Desktop App:** Rust + [Bun](https://bun.sh/) (Tauri prerequisites)
* **Go Package:** Go 1.22+

---

## 📄 License

This project is open-source and available under the [MIT License](LICENSE).
