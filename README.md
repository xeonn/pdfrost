# pdfrost

pdfrost turns a [JasperReports](https://github.com/Jaspersoft/jasperreports) `.jrxml` template and one JSON document into a PDF. JasperReports 6.20.1 is the behavior reference for the tags this subset implements. The same engine is a Rust crate, a Go module, and a desktop app.

JSON is the data source. The caller supplies the template and one JSON object. `$F{name}` reads a property of that object. A database connection is not part of this subset, and one is not planned. pdfrost does not evaluate arbitrary Java.

| Caller | How |
| --- | --- |
| Rust | Crate [`pdfrost-engine`](crates/pdfrost-engine/README.md) |
| Go | Module [`github.com/xeonn/pdfrost/go`](go/README.md) |
| Desktop | Tauri app in this repository |

A tag the subset does not draw is reported as a warning. The PDF still contains the supported content. A template that cannot be parsed returns an error and no PDF. The supported tags are listed in [docs/jrxml-subset.md](docs/jrxml-subset.md).

## Rust

In another project:

```toml
[dependencies]
pdfrost-engine = { git = "https://github.com/xeonn/pdfrost" }
chrono = { version = "0.4", default-features = false, features = ["clock", "std"] }
serde_json = "1"
```

Inside this repository, depend on the path `crates/pdfrost-engine` instead of the git URL.

```rust
use chrono::Utc;
use serde_json::json;
use pdfrost_engine::render;

fn main() {
    let template = std::fs::read_to_string("report.jrxml").expect("template");
    let document = json!({
        "CompanyName": "Northwind Traders",
        "ContactName": "Maria Anders"
    });
    let output = render(&template, &document, Utc::now()).expect("render");
    std::fs::write("report.pdf", &output.pdf).expect("pdf");
    for warning in &output.warnings {
        eprintln!("{} {}", warning.tag, warning.location);
    }
}
```

`document` is one JSON object. `$F{CompanyName}` reads that property. A table bound with `subDataSource("itemsList")` repeats over the array of that name. `Utc::now()` is the instant printed for `new java.util.Date()`, formatted as `YYYY-MM-DD HH:MM:SS` in UTC.

`parse` reads a template into a `Report`. `write_report` writes the supported subset back to JRXML. Details are in [crates/pdfrost-engine/README.md](crates/pdfrost-engine/README.md).

## Go

```bash
go get github.com/xeonn/pdfrost/go@v0.1.0
```

The module is the `go/` directory, so the Git tag is `go/v0.1.0`. That tag is version `v0.1.0` of `github.com/xeonn/pdfrost/go`. The module zip includes a prebuilt `engine.wasm`. The program compiles the Go wrapper only. At runtime the wrapper runs that file with wazero. Cargo is not required.

```go
import (
    "os"
    "time"

    "github.com/xeonn/pdfrost/go"
)

func main() {
    template, _ := os.ReadFile("report.jrxml")
    document := []byte(`{"CompanyName":"Northwind Traders","ContactName":"Maria Anders"}`)
    result, err := pdfrost.Render(string(template), document, time.Now().UTC())
    if err != nil {
        panic(err)
    }
    os.WriteFile("report.pdf", result.PDF, 0o644)
    for _, warning := range result.Warnings {
        println(warning.Message)
    }
}
```

`document` must be a JSON object. The field and table rules match the Rust call. Details, the HTTP sample, and how to rebuild `engine.wasm` are in [go/README.md](go/README.md).

## Examples in this repository

Render a sample template to `sample.pdf`:

```bash
cd examples/go-render
go run .
```

Serve `POST /render` and return a PDF download:

```bash
cd examples/go-server
go run .
curl -sS -D - -o report.pdf --data-binary @../../jrxmls/Sample.jrxml \
  -H 'Content-Type: application/xml' http://127.0.0.1:8080/render
```

Both examples use a `replace` directive so they build against `go/` in this checkout.

## Desktop app

Requirements: Rust, Cargo, [Bun](https://bun.sh/) 1.4, and the platform webview toolchain (Xcode command-line tools on macOS).

```bash
bun install
bun run tauri dev
```

`bun run tauri build` produces a release bundle. Opening a `.jrxml` file starts on Design. The window edits the supported subset there and previews the PDF from the same engine. Preview fills from a JSON document.

## Repository

| Path | Role |
| --- | --- |
| `crates/pdfrost-engine/` | Rust library. Parse, fill, layout, PDF. |
| `crates/pdfrost-wasm/` | WebAssembly build copied to `go/engine.wasm`. |
| `go/` | Go module `github.com/xeonn/pdfrost/go`. |
| `examples/go-render/` | Command-line render. |
| `examples/go-server/` | HTTP `POST /render`. |
| `src-tauri/`, `src/` | Desktop shell and Vue interface. |
| `jrxmls/` | Sample templates. |
| `docs/` | Subset, pipeline, and design notes. |
