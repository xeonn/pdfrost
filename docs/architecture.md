# Architecture

pdfrost is one Rust engine and three callers. The engine owns the report. The Tauri shell owns the window. The TypeScript UI owns presentation. A Go package links the same engine for programs that are not the desktop app.

## Components

```text
jrxml file + JSON document
        |
        v
+-------------------+   +------------------+   +------------------+
| Viewer UI         |   | Go package       |   | Other Rust       |
| Vue 3 + TypeScript |   | go/              |   | program          |
+-------------------+   +------------------+   +------------------+
        |                      |                        |
        | Tauri command        | embedded wasm          | crate dependency
        v                      |                        |
+-------------------+          |                        |
| src-tauri (shell) |          |                        |
+-------------------+          |                        |
        |                      |                        |
        v                      v                        v
+------------------------------------------------------------------+
| Engine crate                                                     |
| parse -> fill -> layout -> PDF                                   |
| Rust API for Rust callers, wasm for the Go package               |
+------------------------------------------------------------------+
        |
        v
   PDF bytes + warnings
```

| Piece | Responsibility |
| --- | --- |
| Engine crate | Parse JRXML, fill from caller-supplied JSON, lay out pages, write PDF bytes, and list unsupported tags. No file dialog, no webview, no database. |
| `src-tauri/` | Desktop process. Exposes commands the window can call. Reads the template and the data file from disk. Depends on the engine crate. Does not reimplement parse, fill, layout, or export. |
| `src/` | The viewer interface: Vue 3, TypeScript, Vite, Bun, and flowbite-vue on Flowbite 4. Asks the shell to render, then shows the PDF and the warning list. |
| `go/` | Go module `github.com/xeonn/pdfrost/go`. Runs the committed `engine.wasm` with wazero. Does not parse JRXML itself. |
| Designer (later) | More of the same window. It edits a template the engine can already parse and previews through the same command. |

Parse, fill, layout, and PDF export stay in the engine crate. The Tauri crate depends on it. Putting the pipeline inside `src-tauri` would make the app the only program that can generate a PDF.

## Suggested layout

```text
crates/pdfrost-engine/    library: template + JSON -> PDF bytes
crates/pdfrost-charts/    category charts in the Chart.js 4.4.6 colors; the engine paints them
crates/pdfrost-wasm/      wasm32 build of that library, copied to go/engine.wasm
go/                       Go module github.com/xeonn/pdfrost/go
examples/go-render/       sample Go program that imports go/
src-tauri/                Tauri shell and commands
src/                      viewer UI
jrxmls/                   acceptance templates
docs/                     these specifications
```

Frontend dependencies stay on Bun (`package.json` declares `packageManager` `bun@1.4.0`). Report logic stays in Rust. The public Go path runs the WebAssembly build described in [Go package](go-package.md).

## Call shape

The shell, a Rust crate, and the Go package use one operation:

- Input: JRXML text (or a path the shell has already read) and one JSON document.
- Output: PDF bytes, plus a list of warnings for tags and expressions the subset does not implement.

A warning does not by itself fail the render. The PDF contains the supported content. The warning names the gap. A template that cannot be parsed, or a supported expression that cannot be filled because the data is missing, is an error and yields no PDF.

The shell's command is a thin wrapper: read the two files, call the engine, return the bytes and the warnings to the webview. The UI does not parse JRXML.

## Why the stages stay inside the crate

Parse, fill, layout, and export are separate steps so each can be tested, and so the viewer and a future designer share one implementation. The public surface can stay a single render call. The stages are internal structure, described in [Report pipeline](report-pipeline.md).

## Process boundary

The webview never links the engine. Tauri commands cross that boundary. PDF bytes come back to the UI for display. Unsupported-tag warnings come back with them, so the window can show what was skipped.
