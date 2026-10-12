# pdfrost

pdfrost makes a PDF from a report design and a JSON file.

The design is a JasperReports template, a `.jrxml` file. The JSON file is the data: names and values such as a company name, a contact, or a list of line items. You do not connect pdfrost to a database. There is no database login, and one is not planned.

A text field such as `$F{CompanyName}` prints the company name from that JSON file. A table or a chart aimed at a list prints one row, or one point, for each entry in the list. A subreport is a second template. It uses the same JSON file: either the whole document, or one list inside it, such as `orders`.

The usual file is one object, such as one invoice. A file that is a list of objects prints the detail band once for each object. Preview reads that list one object at a time, so a long run of invoices does not have to sit in memory all at once. The lines of a single invoice stay with that invoice.

The same program is a Rust library, a Go package, and a desktop app. For the pieces it supports, it follows JasperReports 6.20.1. It does not run Java code written inside a template.

## What it is good for

The engine is a small tool you embed in another system when that system needs to print a receipt, an invoice, or a statement. The other system already has the page design as a JasperReports template and the details of that one document as a JSON file. pdfrost fills the page and returns a PDF. It does not connect to a database.

The samples in this repository are that kind of paperwork: a goods receipt, a shipment, a stock adjustment, and transfers between branches, locators, and warehouses. A Rust or Go program calls the engine and gets the PDF back, with no window of its own. The desktop app is for laying out the page and checking the printout.

## Why a template

The page and the program that prints it are two different jobs. The program knows the facts: the invoice number, the customer, the lines, the total. The page knows where those facts sit, what the headings say, and how the table looks. A template keeps the page in a file. The program only passes the JSON.

If the layout is written into the program, a new heading, a moved logo, or an extra column is a code change. Someone has to edit the print routine, build the system again, and ship it. With a template, that same change is a save of the `.jrxml` file. The program that embeds the engine stays as it is. The next print uses the new page.

One engine can print many documents this way. A receipt, an invoice, and a statement are three template files, not three print routines. The system keeps owning the data. The template keeps owning the paper.

## What this project is not

pdfrost is not a reporting server. It does not keep a shared catalog of reports, run them on a schedule, or give people a place to log in and browse them. For that, use the JasperReports suite. pdfrost is the piece another program calls when it only needs to print one document.

It is not a database tool. It never logs in to a database, and a connection is not planned. It also does not run a query stored in the template. The caller hands over one JSON file for the receipt, invoice, or statement being printed.

It is not the whole JasperReports product. The result is a PDF. It does not produce Excel, Word, or a web page, it does not run Java written inside the template, and it does not compile the template into a `.jasper` file. A chart type, barcode, or other piece outside the supported list is left off the page and named in a warning. The rest of the document still prints. If the template file itself cannot be read, there is no PDF. The supported pieces are listed in [docs/jrxml-subset.md](docs/jrxml-subset.md).

The desktop app is not Jaspersoft Studio. It is a simpler editor for the pieces this engine can print, plus a preview of the PDF.

## What you can do

In the desktop app, opening a template starts on Design. From there you can:

- Place text, a text field, a picture, a line, a box, a chart, a table, or a subreport
- Add or remove the optional bands, such as the title and the page header
- Point a text field at a name in the JSON file
- Repeat a table for each item in a list
- Drop in another template as a subreport, then pick the list it should repeat from a view of the JSON file
- Open Dataset and read the JSON file as a table. Click a list to show one row per object. Read Fields adds those columns to the report’s field list and leaves names that are already there
- Switch to Preview and read the PDF

A chart can be a bar, line, or area chart, including the stacked forms. A picture is a file on your computer. A long subreport continues on the next page.

To see this with the sample files, use File → Open Report and open `jrxmls/Subreport.jrxml`. Then open Dataset and choose `jrxmls/SubreportData.json`. That screen shows the records. Preview is a two-page list of orders. Each order is drawn by `jrxmls/OrderLines.jrxml`, using the `orders` list in that JSON file.

To print a list of receipts across two pages, open `jrxmls/Receipts.jrxml`, then choose `jrxmls/Receipts.json` in Dataset. That file is a list. Each object is one receipt.

| Way to use it | Where |
| --- | --- |
| Rust | Library [`pdfrost-engine`](crates/pdfrost-engine/README.md) |
| Go | Package [`github.com/xeonn/pdfrost/go`](go/README.md) |
| Desktop | The app in this repository |

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

`document` is the JSON for the print. One object is one receipt, invoice, or statement. A list of objects prints the detail band once for each object. `render_read` reads that list from a file, one object at a time. The time you pass in is what the report prints when the template asks for the current date and time. It is shown in UTC, for example `2026-10-12 00:00:00`.

`parse` reads a template so the app can edit it. `write_report` saves that edit back to a `.jrxml` file. Details are in [crates/pdfrost-engine/README.md](crates/pdfrost-engine/README.md).

## Go

```bash
go get github.com/xeonn/pdfrost/go@v0.1.0
```

The module is the `go/` directory, so the Git tag is `go/v0.1.0`. That tag is version `v0.1.0` of `github.com/xeonn/pdfrost/go`. The download includes a ready-made copy of the engine. Building the Go program does not require Rust.

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

`document` must be one JSON object, the same rule as the Rust call. Details, the small web sample, and when the packaged engine is refreshed are in [go/README.md](go/README.md).

## Examples in this repository

Make a sample PDF named `sample.pdf`:

```bash
cd examples/go-render
go run .
```

Start a small web service that accepts a template and returns a PDF:

```bash
cd examples/go-server
go run .
curl -sS -D - -o report.pdf --data-binary @../../jrxmls/Sample.jrxml \
  -H 'Content-Type: application/xml' http://127.0.0.1:8080/render
```

Both examples are wired to the `go/` folder in this checkout, so they do not download a published version.

## Desktop app

You need Rust, Cargo, [Bun](https://bun.sh/) 1.4, and the tools that build a desktop window (Xcode command-line tools on macOS).

```bash
bun install
bun run tauri dev
```

`bun run tauri build` makes an installable app. File → Open Report starts on Design, where you edit the report. Dataset, beside File, is where you choose the JSON file. Preview shows the PDF from that file. The values do not come from a database.

## Repository

| Path | What it is |
| --- | --- |
| `crates/pdfrost-engine/` | The Rust library that reads the template, fills it, and writes the PDF. |
| `crates/pdfrost-wasm/` | The engine packaged so the Go module can run it. |
| `go/` | The Go package `github.com/xeonn/pdfrost/go`. |
| `examples/go-render/` | A command that writes one PDF. |
| `examples/go-server/` | A small web service. Send it a template, and it returns a PDF. |
| `src-tauri/`, `src/` | The desktop app. |
| `jrxmls/` | Sample templates and the JSON files that fill them. |
| `docs/` | Notes on what is supported and how the app is built. |
