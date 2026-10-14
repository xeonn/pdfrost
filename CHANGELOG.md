# Release notes

## 0.3.0

Changes since `go/v0.2.0`. The desktop app, the Rust crates, and the Go module are all 0.3.0. The Go module tag is `go/v0.3.0`.

### Images

- A quoted string or `$F{name}` is still a path, raw base64, or a `data:` URI.
- `new java.io.File(...)` and `new java.net.URL(...)` read the path or address inside the call. `http` and `https` stay unloaded.
- `new java.io.ByteArrayInputStream(...)` around `Base64.getMimeDecoder().decode`, `Base64.getDecoder().decode`, `Base64.decodeBase64`, or `Base64Util.decode` draws the decoded PNG or JPEG. The argument is a quoted string or `$F{name}`, with an optional `.trim()` or `.getBytes()`. A `? null :` branch in front of the constructor is skipped.
- Browsing for a PNG or JPEG stores the bytes on a `pdfrost.` property of that image and sets the expression to `$F{pdfrost.<name>}`. A string field of that name, with no JSON path, is added so Studio can compile. Studio keeps the property. The picture draws here and stays blank in Studio.
- The expression class is String, File, or URL. Choosing File or URL wraps a quoted path or `$F{name}`. Choosing String unwraps that call. Typing one of those constructors sets that class.

### Go package

`go/engine.wasm` is rebuilt for this release. A file path still fails in that build. Base64, a data URI, a `pdfrost.` property, and the constructors above still draw.

```bash
go get github.com/xeonn/pdfrost/go@v0.3.0
```

### Desktop

- Adding an image opens a file dialog for a PNG or JPEG.
- The title bar shows the version, 0.3.0.

## 0.2.0

Changes since `go/v0.1.0`. The desktop app, the Rust crates, and the Go module are all 0.2.0. The Go module tag is `go/v0.2.0`.

### Fill

- A JSON object is still one record. A JSON array is one record per element: the detail band repeats, the title sees the first element, and the summary sees the last. `render_read` reads a root array one element at a time.
- A JSON `queryString` that names a path in the document selects the main records. A missing path leaves the document as it is. `jrxmls/Receipts.jrxml` with `jrxmls/Receipts.json` prints one receipt per object, across two pages.
- A field's JSON expression is stored relative to that query, as Jaspersoft Studio does (`Address` when the query is `Northwind.Customers`, `product.id` on an item). A path that starts with `(root)` still fills.
- A subreport reads a path in the same JSON document, such as `orders` or `customer.orders`. A long subreport continues on the next page.
- Images in the template are drawn in the PDF.

### Go package

`go/engine.wasm` is rebuilt for this release. The package accepts one JSON value: an object is one record, and an array is one record per element, held in memory. It does not stream the array. A JSON `queryString` selects the main records when that path exists.

```bash
go get github.com/xeonn/pdfrost/go@v0.2.0
```

### Desktop

- Design opens the report. The palette adds text, fields, images, lines, boxes, charts, tables, and subreports. Bands can be added and removed from the outline. Drag a tool onto a band to place it.
- File holds New, Open, Save, Save As, and Print. Dataset, beside File, is where the JSON file is chosen. Read Fields appends columns from the query, or from the first list when the root has none, and keeps a field you already added until you remove it.
- Save and Ctrl+S or Cmd+S leave the report open. Preview uses the report on screen, including edits that are not saved. With no JSON file it fills one empty record. Source shows the JRXML for that report before it is written to disk.
- The title bar shows the version, 0.2.0.
