# Go package

Go programs render a template by importing `github.com/xeonn/pdfrost/go`. The package is a wrapper. Parsing, fill, layout, and PDF export stay in `crates/pdfrost-engine`. The Go code does not read JRXML.

The module lives in `go/`, so a release tag is `go/v0.1.0` and another project depends on it with:

```go
require github.com/xeonn/pdfrost/go v0.1.0
```

```bash
go get github.com/xeonn/pdfrost/go@v0.1.0
```

`go get` downloads the module zip, including the committed `engine.wasm`. The first build compiles the Go wrapper only. At runtime the wrapper starts that file with [wazero](https://github.com/tetratelabs/wazero). Cargo is not required.

## Call

```go
import "github.com/xeonn/pdfrost/go"

result, err := pdfrost.Render(templateXML, document, renderedAt)
```

| Argument | Meaning |
| --- | --- |
| `templateXML` | The `.jrxml` text. |
| `document` | One JSON object, the same document the Rust `render` function accepts. |
| `renderedAt` | The UTC instant printed for `new java.util.Date()`. |

`result.PDF` is the file bytes. `result.Warnings` lists unsupported tags and missing fields. `err` is set when the template is not readable JRXML. A warning alone does not set `err`.

A Go service uses this for a batch or a request. It does not start Tauri.

## How Go reaches Rust

The package embeds `go/engine.wasm` and calls it through wazero. That file is a `wasm32-unknown-unknown` build of `crates/pdfrost-wasm`, which calls the same `render` function as the desktop app.

```text
Go pdfrost.Render
    -> wazero
        -> render          (wasm export)
            -> Rust render
```

The host copies the template and the JSON into the module, calls `render`, and reads a length-prefixed blob back. A non-zero first length is an error string. A zero first length is followed by the PDF and the warning strings. `alloc_bytes` and `dealloc_bytes` own that memory.

`rendered_at_unix` is Unix seconds. The engine formats it as `YYYY-MM-DD HH:MM:SS` in UTC, the same rule as the Rust API.

## Build

From `go/`:

```bash
go test ./...
```

That test renders `jrxmls/Sample.jrxml` with the embedded wasm. The category charts are plotted from `chartPoints`, so the result does not carry a `chart` warning.

Rebuild `engine.wasm` after an engine change, then copy it into the module so the next tag ships the new bytes:

```bash
rustup target add wasm32-unknown-unknown
RUSTFLAGS="--remap-path-prefix=${HOME}/=" \
  cargo build --release --target wasm32-unknown-unknown -p pdfrost-wasm
cp target/wasm32-unknown-unknown/release/pdfrost_wasm.wasm go/engine.wasm
```

The remap keeps the home directory of the machine that built the file out of panic text.

`examples/go-render` imports the module through a local `replace`. From that directory:

```bash
go run .
```

It reads `../../jrxmls/Sample.jrxml` and `data.json`, calls `pdfrost.Render`, and writes `sample.pdf`.

`examples/go-server` is an HTTP sample of the same call. `POST /render` with the `.jrxml` as the body returns `application/pdf` and `Content-Disposition: attachment`, so the client downloads the file. Optional JSON data can be a multipart field named `data`, or a JSON body `{"template","data"}`. From that directory:

```bash
go run .
curl -sS -D - -o report.pdf --data-binary @../../jrxmls/Sample.jrxml \
  -H 'Content-Type: application/xml' http://127.0.0.1:8080/render
```

## What the package does not do

- Parse or lay out a report in Go.
- Open a window.
- Accept a SQL connection, a file path, or more than one JSON document. The caller reads the files and passes the bytes.
