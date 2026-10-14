# github.com/xeonn/pdfrost/go

This is the Go package for pdfrost. Pass it a report template and one JSON value, and it returns a PDF.

The JSON object is the data. pdfrost does not open a database connection, and one is not planned. A text field reads one value from the object. A table or a chart can walk a list inside it. The work of reading the template and laying out the page stays in the Rust engine. This package carries a ready-made copy of that engine and runs it with [wazero](https://github.com/tetratelabs/wazero).

Use it from a Go service that needs to print a receipt, an invoice, or a statement. The service already has the page design and the details of one document as JSON, and the call returns a PDF. There is no window and no database connection. The samples are that kind of paperwork: a goods receipt, a shipment, a stock adjustment, and transfers between branches, locators, and warehouses. For a full reporting server, use the JasperReports suite.

## Install

The module is this directory. A release is the Git tag `go/v0.3.0`, which `go get` sees as version `v0.3.0`:

```bash
go get github.com/xeonn/pdfrost/go@v0.3.0
```

```go
require github.com/xeonn/pdfrost/go v0.3.0
```

In a checkout of this repository, point a `replace` at `go/` instead of fetching a tag:

```go
require github.com/xeonn/pdfrost/go v0.0.0

replace github.com/xeonn/pdfrost/go => ../../go
```

`go build` compiles this package only. It does not build the Rust engine.

## Render

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

| Argument | Meaning |
| --- | --- |
| template | The text of the `.jrxml` file. |
| document | One JSON value. An object is one record. An array is one record per element. `$F{CompanyName}` prints `Northwind Traders`. A table aimed at `itemsList` prints one row for A-1. |
| `time.Now().UTC()` | The date and time printed when the template asks for the current time. It is shown in UTC, for example `2026-10-12 00:00:00`. |

`result.PDF` is the file. `result.Warnings` lists anything the template asked for that was skipped, including a missing value. A warning does not cancel the PDF. `err` is set when `document` is not JSON, or when the template cannot be read. There is no PDF in that case.

## Samples

From `examples/go-render`:

```bash
go run .
```

That reads `../../jrxmls/Sample.jrxml` and `data.json`, and writes `sample.pdf`.

From `examples/go-server`:

```bash
go run .
curl -sS -D - -o report.pdf --data-binary @../../jrxmls/Sample.jrxml \
  -H 'Content-Type: application/xml' http://127.0.0.1:8080/render
```

`POST /render` accepts the template on its own, a JSON body with the template and the data together, or a form with fields named `template` and `data`. The response is a PDF download. Warnings are repeated in the `X-Pdfrost-Warning` header.

## Tests and the packaged engine

```bash
cd go && go test ./...
```

The test fills `jrxmls/Sample.jrxml` through the packaged engine. The charts use the `chartPoints` list in the sample data, so that run does not warn about a chart.

`go/engine.wasm` is that packaged engine. It is replaced only when a release of this Go package is being prepared. Day-to-day engine changes do not refresh it. This copy accepts one JSON value: an object is one record, and an array is one record per element, held in memory. The Rust library's `render_read` still streams a root array one element at a time. When a release is being prepared, rebuild the file and commit it with the tag:

```bash
rustup target add wasm32-unknown-unknown
RUSTFLAGS="--remap-path-prefix=${HOME}/=" \
  cargo build --release --target wasm32-unknown-unknown -p pdfrost-wasm
cp target/wasm32-unknown-unknown/release/pdfrost_wasm.wasm go/engine.wasm
```

The remap keeps the home directory of the machine that built the file out of error text.
