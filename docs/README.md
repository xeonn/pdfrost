# Design specifications

These documents describe the pdfrost system for this repository.

Read them in this order:

| Document | What it decides |
| --- | --- |
| [System overview](system-overview.md) | What pdfrost is, the subset of JasperReports it covers, and the outputs. |
| [Go package](go-package.md) | The Go module that runs the embedded wasm engine. Usage is in [`go/README.md`](../go/README.md). |
| [Architecture](architecture.md) | How the engine crate, the Tauri shell, and the window divide the work. |
| [Report pipeline](report-pipeline.md) | Parse, fill, layout, and PDF export, and the library call that runs them. |
| [JRXML subset](jrxml-subset.md) | Which tags and expressions are supported, ignored, or reported. |
| [Charts](charts.md) | Category charts plotted in the Chart.js 4.4.6 color scheme. |
| [Viewer](viewer.md) | Preview-shaped window: open JRXML, empty data, PDF stage, Design placeholder. |
| [Designer](designer.md) | Studio-shaped Design tab: band canvas, outline, palette, properties, save through the engine. |
| [Viewer and designer](viewer-and-designer.md) | Short note on the viewer pass and where the designer plan sits. |
| [Milestones](milestones.md) | The order of delivery, checked against the files in `jrxmls/`. |

JasperReports 6.20.1 is the behavior reference for the subset. It stays outside this repository. See [System overview](system-overview.md#reference-project).
