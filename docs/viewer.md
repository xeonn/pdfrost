# Viewer plan

Build the desktop Preview screen so it reads like Jaspersoft Studio’s **Preview** tab. The designer is only a placeholder on the same window. Band canvas, palette, outline, and Source XML stay out of this plan.

The engine already returns PDF bytes and warnings. This plan is the Flowbite shell around that render, and the PDF.js page stage.

## Goal

1. Open a `.jrxml` file.
2. Render it at once with an empty JSON object `{}`.
3. Show the PDF like Studio Preview: gray stage, white page, page and zoom controls.
4. List every warning. `Sample.jrxml` plots its charts when `chartPoints` is present, so that preview has no `chart` warning. Empty JSON still draws the frames and warns `chart`.
5. Optionally choose a JSON document and render again with that data.
6. After a template is loaded, switch between **Preview** and **Design**. Design is a placeholder until a later plan.

## Reference

Studio Preview (not Design):

- A preview toolbar: first / previous / next / last page, **Page N of M**, zoom out / in, zoom percent, fit width, fit page.
- The page as a white sheet on a gray background, one page at a time.
- Mode tabs for the open report: Design / Preview (Studio also has Source; we omit Source).
- A **Report state** strip under the page for warnings and errors (Studio’s Console / Errors / Statistics).

Data-adapter dropdowns, language, and a separate Run control are not part of this plan. Opening the JRXML is the run. Choosing JSON is a refill.

## Layout

Full-height app shell. Flowbite owns chrome. PDF.js owns the page.

```text
┌──────────────────────────────────────────────────────────┐
│ Top bar: Open JRXML · Choose JSON · data badge · spinner │
├──────────────────────────────────────────────────────────┤
│ Preview toolbar (only when a PDF is showing)             │
│   « ‹  Page N of M  › »   −  100%  +   Fit width / page  │
├──────────────────────────────────────────────────────────┤
│                                                          │
│              gray stage · white page (PDF.js)            │
│                                                          │
├──────────────┬───────────────────────────────────────────┤
│ Design | Preview   Report state: Warnings · Error        │
└──────────────┴───────────────────────────────────────────┘
```

Before a template loads, the center is an empty state with one primary action: **Open JRXML**. The Design / Preview switch is hidden.

After a successful render, Design / Preview appear. Switching mode does not re-render. Both modes keep the same template path and the same JSON path (or empty data).

### Flowbite map

| Need | Piece |
| --- | --- |
| Top actions | `FwbButton`, optional navbar |
| Data state | `FwbBadge`: Empty data / Filled |
| Loading | `FwbSpinner` |
| Page / zoom | `FwbButton` or `FwbButtonGroup` |
| Design / Preview | `FwbTabs` |
| Warnings / errors | `FwbAlert` in Report state |
| Designer placeholder | Quiet empty state or info alert |

## Behaviour

### Open template

Choosing a `.jrxml` calls `render_report` with that path and **no document path**. The shell passes `{}` to the engine. The PDF and warnings appear. Fields that need data print blank and may warn; static labels, lines, table headers, and the page chrome still show.

A file that is not JRXML returns an error, clears the PDF, and keeps Design / Preview hidden.

### Choose JSON

Optional. Choosing a `.json` file renders again with that document. The badge switches from Empty data to Filled. The template path does not change. Clearing back to empty data may wait; this plan only requires open-with-empty and choose-JSON.

### Preview stage

- One page at a time from the current PDF bytes.
- Zoom steps and a percent label; fit width and fit page against the stage size.
- Previous / next / first / last; show **Page N of M**.
- While rendering, keep the last good page when possible and show the spinner. On failure, clear the page and show the error in Report state.

Do not iframe the PDF. Do not stack every page as a long scroll of canvases.

### Design placeholder

When Design is selected:

- Same top bar and same loaded template name.
- Center message: editing is not available yet.
- No property form, no band canvas, no save from this placeholder.

Saving a template belongs to the designer plan. This viewer plan does not expose it in the UI.

### Report state

Always show warnings from the last successful render. A parse or render failure shows as an error with no document. Sample’s empty-data preview draws the chart frames and lists a `chart` warning because `chartPoints` is missing. A preview filled from `fixtures/sample.json` or `SampleData.json` plots the five charts and does not list that warning.

## Shell changes

`render_report` today requires a document path. Change it so the document path is optional. When it is missing or empty, pass `serde_json::json!({})` into `pdfrost_engine::render`.

The webview still does not parse JRXML. It sends paths (or later, contents) and displays PDF bytes and warnings.

## Build order

1. Optional document path on `render_report`; empty object when absent.
2. App shell: top bar, center stage, Report state strip.
3. Open JRXML → render with `{}` → show Design / Preview switch.
4. Replace stacked canvases with one-page PDF.js drawing, page controls, zoom, fit width / fit page.
5. Design placeholder in the same shell (remove the current form from that slot).
6. Choose JSON to refill.

## Done when

With `bun run tauri dev`:

1. Open `jrxmls/Sample.jrxml` with no JSON. The page shows the static title, the headings, and the five chart frames. Report state lists a `chart` warning. Badge says Empty data.
2. Page and zoom controls work on that PDF.
3. Switch to Design: placeholder only. Switch back to Preview: same PDF, no second render required.
4. Choose `jrxmls/fixtures/sample.json` (or `jrxmls/SampleData.json`). The page refills. Badge says Filled.
5. Open `jrxmls/ShipmentOut.jrxml` with empty data, then choose `jrxmls/fixtures/shipment-out.json`. Multi-page navigation works.

## Out of this plan

- Band canvas, palette, outline, properties grid
- Source XML tab
- Project explorer / multi-file tabs
- Print, export-as, and Studio data adapters
- Images, stretch-with-overflow, Page X of Y (engine gaps; a new screen will not draw them). Category charts are already in the PDF.
- Implementing the real designer (placeholder only)

## Task list

- [x] Optional document path on `render_report`; pass `{}` when absent
- [x] App shell: top bar, center stage, Report state strip
- [x] Open JRXML → render with empty data → show Design / Preview switch
- [x] One-page PDF.js stage with page controls, zoom, fit width / fit page
- [x] Design placeholder in the same shell (no property form)
- [x] Choose JSON to refill; Empty data / Filled badge
- [x] Done when: Sample with empty data shows the title and the area chart
- [x] Done when: page and zoom controls work
- [x] Done when: Design placeholder, then back to Preview without re-render
- [x] Done when: Sample with a JSON fixture refills
- [x] Done when: ShipmentOut empty, then filled, multi-page navigation works
