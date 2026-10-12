# Design notes

These notes describe how pdfrost is built. pdfrost makes a PDF from a report template and a JSON file. That JSON file is the data. pdfrost does not connect to a database, and a database connection is not planned.

The engine is a small tool another system embeds to print a receipt, an invoice, or a statement from a template and one JSON file. It is not a full reporting server. For a shared catalog, schedules, and database queries, use the JasperReports suite. The samples here are a goods receipt, a shipment, a stock adjustment, and transfers between branches, locators, and warehouses.

Read them in this order:

| Document | What it covers |
| --- | --- |
| [System overview](system-overview.md) | What pdfrost is, which parts of JasperReports it covers, and the three ways to use it. |
| [Go package](go-package.md) | The Go package that runs the packaged engine. How to call it is in [`go/README.md`](../go/README.md). |
| [Architecture](architecture.md) | How the library, the desktop shell, and the window split the work. |
| [Report pipeline](report-pipeline.md) | How a template and a JSON file become a PDF, and the library call that does it. |
| [JRXML subset](jrxml-subset.md) | Which parts of a template are drawn, kept, or reported as a warning. |
| [Charts](charts.md) | Bar, line, and area charts, drawn in the Chart.js 4.4.6 colors. |
| [Viewer](viewer.md) | The Preview screen: open a template, choose a JSON file, and read the PDF. |
| [Designer](designer.md) | The Design screen: the page, the outline, the tool palette, the properties, and saving. |
| [Viewer and designer](viewer-and-designer.md) | A short note on how Preview and Design share one window. |
| [Milestones](milestones.md) | The order the work was delivered, checked against the files in `jrxmls/`. |
| [Release notes](../CHANGELOG.md) | What changed in each version, starting at 0.2.0. |

JasperReports 6.20.1 is the guide for how a supported piece should behave. That project is not copied into this repository. See [System overview](system-overview.md#reference-project).
