# Charts

Plot category charts inside the `reportElement` box. JasperReports 6.20.1 is the behavior reference. `jrxmls/Sample.jrxml` is the first template: its summary band holds one of each supported chart.

`areaChart`, `barChart`, `lineChart`, `stackedAreaChart`, and `stackedBarChart` are parsed, filled, and painted. `Sample.jrxml` draws all five from `chartPoints` and does not warn `chart` when that array is present. Design can drop a bar chart from the palette and edit its kind, title, legend, evaluation time, data array, and series. Any other chart tag keeps the gray placeholder and the `chart` warning.

## Recommendation

Paint the plot in `crates/pdfrost-charts`. JasperReports 6.20.1 draws charts with JFreeChart 1.0.19. No Rust chart crate reproduces that look and also takes the font bytes this engine already embeds. `charts-rs`, `plotters`, `charton`, and `plotkit` each ship their own theme and their own text stack, and each one returns a finished SVG, PNG, or PDF page. `pdfrost-charts` takes the useful split from those crates — a chart value in, a drawing surface out. Category slots and type sizes stay with the JasperReports category plot. The colors are the Chart.js 4.4.6 defaults.

The caller passes the font. pdfrost passes Liberation Sans, the face the rest of the page embeds. JFreeChart asks for Tahoma at 20 pt bold for the title and 12 pt for ticks and the legend. Those sizes stay. The outlines are the supplied face.

`draw` writes through a `Canvas` trait: rectangles, polygons, lines, and text, in the chart box, origin at its top-left. The engine implements that trait on the krilla surface when the plot slice lands. The window and the Go package still call `render`. They do not learn a chart type of their own.

Three choices follow from the engine that already exists.

**The chart is data on `Chart`, filled like a text field, painted like a rectangle.** Parse stores the box, the element tag, the dataset, and the title. Fill turns expressions into category points. Layout keeps the box where the template put it. Export strokes and fills the plot in that box. The Tauri shell and the Go package keep calling `render`. They do not learn a chart type of their own.

**Multi-point data uses `subDataSource`, the same call a table uses.** A chart with no dataset run evaluates each series once against the current main record. An object has one main record. An array has one main record per element. A chart whose dataset run is `subDataSource("…")` evaluates category and value once per object in that array. The operational templates use `itemsList` for tables. A chart can use the same array. `Sample.jrxml` uses `subDataSource("chartPoints")` and two series, `Units` and `Returns`, over the months in that array.

**Ship one kind that `Sample.jrxml` can show, then the kinds that share its dataset.** `areaChart` is first. `barChart` and `lineChart` use the same `categoryDataset` and a different plot. Stacked area and stacked bar come after the plain shapes. A kind this plan does not draw keeps today's placeholder and the `chart` warning.

## Goal

1. Parse the category dataset on `Sample.jrxml` and write it back, so a save keeps the series.
2. Fill those expressions from the JSON document.
3. Draw an area plot in the 200×200 box in the summary band.
4. Draw bar and line plots from the same points.
5. Leave every other chart tag on the placeholder path.

## What Sample stores

```xml
<areaChart>
  <chart evaluationTime="Report">
    <reportElement x="20" y="20" width="200" height="200"/>
    <chartTitle/>
    <chartSubtitle/>
    <chartLegend/>
  </chart>
  <categoryDataset>
    <categorySeries>
      <seriesExpression><![CDATA["SERIES 1"]]></seriesExpression>
      <categoryExpression><![CDATA[0]]></categoryExpression>
      <valueExpression><![CDATA[0]]></valueExpression>
    </categorySeries>
  </categoryDataset>
  <areaPlot>
    <plot/>
    <categoryAxisFormat><axisFormat/></categoryAxisFormat>
    <valueAxisFormat><axisFormat/></valueAxisFormat>
  </areaPlot>
</areaChart>
```

`evaluationTime="Report"` means JasperReports paints the chart after the report has been filled, so every incremented point is present. This engine has one main record, and a `subDataSource` array is already complete when the band is drawn, so `Now` and `Report` fill the same way. Any other `evaluationTime` is stored, warned as `evaluationTime=…`, and filled the same way.

`isShowLegend` defaults to true, which is the JasperReports default. `false` hides the legend. `chartLegend` on Sample is empty. It does not change that default. Position and colors on `chartLegend` wait.

`customizerClass`, `renderType`, `theme`, and `evaluationGroup` are warned when they are set. The plot is still drawn from the dataset.

## Model

`Chart` gains the dataset. The box and `kind` stay.

| Field | Source |
| --- | --- |
| `kind` | Element tag: `areaChart`, `barChart`, `lineChart`, `stackedAreaChart`, `stackedBarChart`. |
| `element` | `reportElement` inside `<chart>`, including `box` when the template has one. |
| `evaluationTime` | Attribute on `<chart>`. |
| `title`, `subtitle` | Raw `titleExpression` and `subtitleExpression` text. An empty `chartTitle` stores nothing. |
| `showLegend` | `isShowLegend`. Missing attribute means true. |
| `dataSource` | The name inside `subDataSource("…")` on the chart's `datasetRun`. |
| `series` | One entry per `categorySeries`: `seriesExpression`, `categoryExpression`, `valueExpression`, stored raw. |

`categoryDataset`, `dataset`, `areaPlot`, `barPlot`, `linePlot`, `plot`, `categoryAxisFormat`, `valueAxisFormat`, and `axisFormat` are recognized so they do not each add a warning. Axis format, `seriesColor`, and label expressions are read in a later slice. Until then, `labelExpression` and `itemHyperlink` warn and the point is still kept.

A chart tag with no `categoryDataset` (`pieChart` and the rest) still warns `chart` and still has an empty series list. The placeholder uses that empty list.

`serde` field names stay camelCase, with defaults, so a `Chart` JSON object from before this work still loads.

## Fill

Chart expressions are their own small evaluator. Text fields do not grow new forms because of this.

| Expression | Result |
| --- | --- |
| `"SERIES 1"` | The string `SERIES 1`. One pair of quotes is stripped. |
| `0`, `1.5` | That number. |
| `$F{name}` | The field on the current record, with the same JSON path and the same blank-plus-warning rule as a text field. |
| Anything else | A warning, and that point is skipped. |

Each `categorySeries` is one series.

- With no `dataSource`, evaluate the three expressions once against the main JSON object.
- With a `dataSource`, evaluate `categoryExpression` and `valueExpression` once per object in that array. Evaluate `seriesExpression` on each row too. Points that share a series name stay on one series, in the order the names first appear. Categories stay in the order they first appear.
- A value that is not a number skips that point and warns.
- The same series and category seen again adds to the previous value. That matches category-dataset increment in JasperReports 6.20.1. Confirm against `JRFillCategoryDataset` when the first fixture has a repeated category.

`Sample.jrxml` evaluates `Units` and `Returns` once per object in `chartPoints`. With no `chartPoints` array the dataset is empty, the frame is still drawn, and the element warns `chart`.

## Draw

The box origin and size are the ones layout already computes. The engine calls `pdfrost_charts::draw` with that width and height, the filled `CategoryChart`, and Liberation Sans regular and bold. The canvas adds the box origin on the page.

The engine calls `pdfrost_charts::draw` with Liberation Sans and paints the marks into the element box.

- Chart background white. Plot background white. Gridlines and the axis frame are `rgba(0, 0, 0, 0.1)` on white (`#E6E6E6`). Horizontal lines sit on the value ticks. Vertical lines sit on category centers for area and line charts, and between the category slots for bar charts.
- Title at 20 pt bold when the title expression fills to a non-empty string. Subtitle at 12 pt under it. Title, ticks, and legend text are `#666`.
- Legend along the bottom when `showLegend` is true: an 8 pt swatch and the series name at 12 pt.
- Value labels in the left gutter, category labels under the plot, both 12 pt in `#666`.
- Category slots follow `CategoryAxis`: 5% lower margin, 5% upper margin, and 20% of the length shared as the gap between categories. A value sits at the middle of its slot.
- The value range includes 0. A flat zero series uses 0 to 1. Ticks snap to 1, 2, or 5 times a power of ten.
- A repeated series and category adds to the stored value.

| Kind | Mark |
| --- | --- |
| `areaChart` | A filled polygon from the baseline through each value and back, with a 3 pt line along the top. |
| `barChart` | Side-by-side bars. `BarRenderer`'s item margin is 0.20. |
| `lineChart` | A 3 pt polyline through the slot centers, with a 6 pt square on each point. |
| `stackedAreaChart`, `stackedBarChart` | The same marks, after each value is added to the earlier series at that category. |

Series colors follow the Chart.js 4.4.6 colors plugin: `#36A2EB`, `#FF6384`, `#FF9F40`, `#FFCD56`, `#4BC0C0`, `#9966FF`, `#C9CBCF`, then the list repeats. Bars and areas fill that color at 50% alpha. Lines, area ridges, and line-chart points use the opaque color. The legend swatch matches the fill, except on a line chart, where it matches the line.

A supported kind with no points still draws the frame, the title, and empty axes, and warns `chart` so the empty dataset is visible. A supported kind with at least one point does not warn `chart`. The placeholder label goes away for that element. `PlacementKind::Chart` remains, and `text` becomes the chart kind so a test can find the box.

`Sample.jrxml` draws each supported kind from the same four months. The legend names `Units` and `Returns`. The titles are `Area`, `Bar`, `Line`, `Stacked area`, and `Stacked bar`.

## Writer

`write_report` emits the tag from `kind`, the `reportElement`, `evaluationTime` when set, `isShowLegend="false"` when the legend is off, title and subtitle expressions when set, `chartLegend` when the legend is on, `categoryDataset` and each `categorySeries`, and `subDataSource("…")` when `dataSource` is set. `areaChart` and `stackedAreaChart` get an `areaPlot`. `barChart` and `stackedBarChart` get a `barPlot`. `lineChart` gets a `linePlot`.

Parse of that XML returns the same series, box, and flags. Sample's studio properties and the empty plot chrome stay omitted, which is the same lossy save the designer already confirms when warnings are non-empty. After the area plot is real, Sample's `chart` warning is gone, and that confirm no longer names `chart` for Sample.

## Design

The canvas keeps a box labeled with the chart kind. The palette Chart tool drops a 200×200 bar chart with evaluation time Report and one series (`"SERIES 1"`, category `0`, value `0`). Properties edit the kind, title, subtitle, legend, evaluation time, the `subDataSource` array name, and the series expressions. A named array creates a subdataset and adds each `$F{name}` the series uses. Delete removes the chart.

Preview is where the plot appears. Design does not fill `$F{…}`.

## Warnings that remain

| Situation | Warning |
| --- | --- |
| Kind outside this plan (`pieChart`, XY, time series, meter, 3D, and the rest) | `chart`, placeholder box |
| `evaluationTime` other than `Now` or `Report` | `evaluationTime=…`, plot still drawn when the kind is supported |
| `customizerClass`, `renderType`, `theme`, `evaluationGroup` | that attribute name |
| Expression this evaluator does not accept | the expression text, point skipped |
| Value that is not a number | the expression text, point skipped |
| Supported kind and no points | `chart` |

## Build order

These steps are in the tree. `Sample.jrxml` plots area, bar, line, stacked area, and stacked bar from `chartPoints` and does not warn `chart` when that array is present.

1. **Model, parse, and write.** `Sample.jrxml` keeps `evaluationTime`, the one series (`"SERIES 1"`, `0`, `0`), and `showLegend`. A chart with `subDataSource("itemsList")` keeps that name. `write_report` round-trips those fields. The PDF is still the gray box. The `chart` warning stays. JSON round-trip of every template still passes.
2. **Expressions.** A unit test fills a string literal, a number, and `$F{…}` on the main record and on one `subDataSource` row. No drawing yet.
3. **Area plot.** Call `pdfrost-charts` from the engine with Liberation Sans. Sample draws the frame and the `SERIES 1` legend. The `chart` warning for that element is gone. Update the tests and the docs that currently require the Sample warning: `tests/templates.rs`, `tests/static_pdf.rs`, `tests/positions.rs`, `tests/parse.rs`, [JRXML subset](jrxml-subset.md), [Report pipeline](report-pipeline.md), [Milestones](milestones.md), [Viewer](viewer.md), [System overview](system-overview.md), and the Go package notes that mention the chart warning.
4. **Bar and line.** The crate already paints them. One engine fixture each checks the PDF.
5. **Stacked area and stacked bar.** The crate already stacks. A two-series fixture checks that the second series sits on top of the first.
6. **Design label.** The box reads the tag, and Properties lists the series expressions. Save of Sample writes the dataset.

## Done when

1. `Sample.jrxml` renders area, bar, line, stacked area, and stacked bar in the summary. With `chartPoints` present, the PDF has no `chart` warning. The legend reads `Units` and `Returns`.
2. A fixture with two categories and values 1 and 3 produces an area that rises. A bar fixture and a line fixture of the same points produce those marks.
3. A fixture whose dataset run is `subDataSource("itemsList")` plots one point per object in that array.
4. `pieChart` still renders the gray box and the `chart` warning.
5. `write_report` on Sample, parsed again, still has the series and the chart boxes.
6. `go test ./...` follows the engine: Sample no longer fails for lack of a `chart` warning.

## Out of this plan

- Pie, 3D, XY, scatter, bubble, time series, high-low, candlestick, meter, thermometer, Gantt, spider, and multi-axis charts
- Chart customizers, themes, hyperlinks, and `renderType` of image or svg
- Axis masks, tick fonts, plot orientation, and `seriesColor` (the default palette covers color until a template needs its own)
- JFreeChart's gradient bar painter, and embedding Tahoma itself. Fills use the Chart.js color at 50% alpha. The face is the caller's, at 20 pt bold for the title and 12 pt for ticks and the legend
- Page X of Y, images, and the other gaps already listed in [JRXML subset](jrxml-subset.md)

## Task list

- [x] `crates/pdfrost-charts` paints area, bar, line, and stacked charts in the Chart.js 4.4.6 colors, measuring a caller-supplied font
- [x] Store category series, title, subtitle, legend, evaluation time, and `subDataSource` on `Chart`
- [x] Write that dataset back from `write_report`
- [x] Evaluate string literals, number literals, and `$F{…}` for chart expressions
- [x] Draw `areaChart` and drop the Sample `chart` warning
- [x] Draw `barChart` and `lineChart`
- [x] Draw `stackedAreaChart` and `stackedBarChart`
- [x] Show the tag and the series expressions on the Design tab
- [x] Point the subset, pipeline, viewer, and milestone docs at a plotted chart
