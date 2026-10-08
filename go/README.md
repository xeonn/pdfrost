# github.com/xeonn/pdfrost/go

Go wrapper around the pdfrost engine. Import this module, pass a JRXML template and one JSON object, and receive PDF bytes. Parsing and layout stay in the Rust engine. This package embeds a prebuilt `engine.wasm` and runs it with [wazero](https://github.com/tetratelabs/wazero).

## Install

The module is this directory. A release is the Git tag `go/v0.1.0`, which `go get` sees as version `v0.1.0`:

```bash
go get github.com/xeonn/pdfrost/go@v0.1.0
```

```go
require github.com/xeonn/pdfrost/go v0.1.0
```

In a checkout of this repository, point a `replace` at `go/` instead of fetching a tag:

```go
require github.com/xeonn/pdfrost/go v0.0.0

replace github.com/xeonn/pdfrost/go => ../../go
```

`go build` compiles this package only. It does not run Cargo.

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
| template | The `.jrxml` text. |
| document | One JSON object. `$F{name}` reads a property. A table `subDataSource("itemsList")` repeats over the array `itemsList`. |
| `time.Now().UTC()` | The instant printed for `new java.util.Date()`, as `YYYY-MM-DD HH:MM:SS` in UTC. |

`result.PDF` is the file. `result.Warnings` lists unsupported tags, unsupported expressions, and missing fields. A warning does not fail the call. `err` is set when `document` is not JSON, or when the template cannot be parsed. There is no PDF in that case.

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

`POST /render` accepts the raw `.jrxml` body, a JSON body `{"template","data"}`, or multipart fields `template` and `data`. The response is `application/pdf` with `Content-Disposition: attachment`. Warnings are repeated in `X-Pdfrost-Warning`.

## Tests and rebuilding the engine

```bash
cd go && go test ./...
```

The test renders `jrxmls/Sample.jrxml` through the embedded wasm. The category charts are plotted from `chartPoints`, so the result does not carry a `chart` warning.

After changing the Rust engine, rebuild the blob this module embeds and commit the new file with the tag:

```bash
rustup target add wasm32-unknown-unknown
RUSTFLAGS="--remap-path-prefix=${HOME}/=" \
  cargo build --release --target wasm32-unknown-unknown -p pdfrost-wasm
cp target/wasm32-unknown-unknown/release/pdfrost_wasm.wasm go/engine.wasm
```

The remap keeps the home directory of the machine that built the file out of panic text.
