# JRXML subset

This is the tag and expression contract for the engine. JasperReports 6.20.1 is the behavior reference for every supported tag. The files in `jrxmls/` are the templates the contract was taken from.

Three outcomes are possible for a tag:

| Outcome | Meaning |
| --- | --- |
| Supported | Parsed, and it changes the PDF. |
| Parsed and ignored | Recognized so the file is not reported as broken. It does not change the PDF. |
| Unsupported | Named in the warning list. It is not drawn and it does not fail the render by itself. |

A tag that appears in none of these lists is unsupported.

## Supported tags

| Tag | Effect |
| --- | --- |
| `jasperReport` | Page size, margins, and the report name. |
| `style` | Named style for a table cell: mode, backcolor, and box. |
| `box`, `pen`, `topPen`, `leftPen`, `bottomPen`, `rightPen`, `graphicElement` | Borders. A `graphicElement` pen is the element's pen when the element has no `box` pen. |
| `field`, `fieldDescription` | A printable field. The description is stored with it. |
| `property` | Only `net.sf.jasperreports.json.field.expression`, the JSON path for that field. |
| `background`, `title`, `pageHeader`, `columnHeader`, `detail`, `columnFooter`, `summary`, `pageFooter`, `band` | Bands and the band body. `columnFooter` is painted above `pageFooter` on every page, and its height is kept clear of the flowing bands. |
| `reportElement` | `x`, `y`, `width`, `height`, `mode`, `backcolor`. |
| `staticText`, `text` | Fixed text. |
| `textField`, `textFieldExpression` | A value from a supported expression. |
| `textElement`, `font`, `paragraph` | Alignment, bold, italic, underline, strike-through, line spacing, rotation, markup, and font size. |
| `line` | A rule. |
| `rectangle` | A box. |
| `ellipse` | An oval in the element box. An opaque mode fills the oval. |
| `image`, `imageExpression` | A PNG or JPEG in the element box. The expression is a quoted path or `$F{name}`. A path is read beside the template, then from the process directory. `scaleImage`, `hAlign`, `vAlign`, `rotation`, `isUsingCache`, `isLazy`, `onErrorType`, `evaluationTime`, and `evaluationGroup` are kept. A missing picture warns when `onErrorType` is `Error`, draws nothing when it is `Blank`, and draws a small mark when it is `Icon`. Hyperlinks on the image are warned and omitted. |
| `elementGroup` | Children are laid out on the parent band. The group itself has no box. |
| `break` | `type="Page"` or `type="Column"`. In `title`, `detail`, or `summary`, either starts the next page and later elements in that band use the new page. In a header, footer, or `background`, the tag is kept and does not split the page. |
| `subreport`, `subreportExpression` | Another `.jrxml`, filled into the element box. The expression is a quoted path or `$F{name}`, read beside the master the way an image path is. A `.jasper` path reads the `.jrxml` with the same stem and warns. The band grows with the filled result. The whole block moves to the next page when it does not fit. With no data source, the file is filled once from the current JSON object. `subDataSource("…")` repeats the child's detail band over that array. A table or chart inside the child reads the current element of that array. `connectionExpression`, `subreportParameter`, and `returnValue` warn and are skipped. `Subreport.jrxml` fills `company` and `contact` from `SubreportData.json` and repeats `OrderLines.jrxml` over `orders`. Each order prints `orderNo` and a table of that object's `lines`. |
| `areaChart`, `barChart`, `lineChart`, `stackedAreaChart`, `stackedBarChart` | A category plot in the element box. See [Charts](charts.md). |
| `subDataset` | The record shape a table repeats. |
| `componentElement` | Wrapper around a table. |
| `jr:table`, `jr:column` | The table and one column. |
| `jr:tableHeader`, `jr:columnHeader`, `jr:detailCell`, `jr:columnFooter`, `jr:tableFooter` | Table sections used by the samples. |
| `datasetRun` | Binds the table to a `subDataset`. |
| `dataSourceExpression` | Only `subDataSource("…")`. On a table or chart the string names a JSON array on the current object. The operational templates use `itemsList`. On a subreport the same call is a path: `orders`, or `customer.orders`. A numeric segment indexes an array. |

## Supported expressions

Inside `textFieldExpression`:

| Form | Result |
| --- | --- |
| `$F{name}` | That field on the current record. |
| `$V{PAGE_NUMBER}` | Current page number. |
| `new java.util.Date()` | Time of this render. |

Other expression text is unsupported even when the surrounding tag is supported. That includes `$P{…}` and any Java call other than the `subDataSource` form.

## Parsed and ignored

| Tag | Why it is ignored |
| --- | --- |
| `property` | Keys under `com.jaspersoft.studio.*`. Designer metadata. The JSON field-path property is supported instead. |
| `queryString` | A JSON query declaration. The caller passes the document. The engine does not run it. |
| `group`, `groupExpression` | Groups Studio stored on a `subDataset`. The samples have no `groupHeader` or `groupFooter`. |

## Category charts

`areaChart`, `barChart`, `lineChart`, `stackedAreaChart`, and `stackedBarChart` are plotted in the `reportElement` box. The series comes from `categoryDataset` / `categorySeries`. A `subDataSource("…")` on the chart's `datasetRun` supplies one point per object in that array. With no dataset run, each series is evaluated once against the main JSON object.

Chart expressions are a string literal (`"SERIES 1"`), a number literal (`0`, `1.5`), or `$F{name}`. Anything else warns and that point is skipped. `evaluationTime` of `Now` or `Report` fills the same way. Any other value warns and still fills.

`Sample.jrxml` places one of each supported chart in the summary. Each chart reads `subDataSource("chartPoints")` on the `ChartPoints` dataset: category `$F{month}`, series `Units` from `$F{units}`, and series `Returns` from `$F{returns}`. `jrxmls/SampleData.json` and `jrxmls/fixtures/sample.json` both carry that array for January through April. Filled that way, the plots do not warn `chart`. An empty document still draws the frames and warns `chart`.

A chart tag outside that list, such as `pieChart`, keeps the gray placeholder labeled Chart and warns with tag `chart`. Axis format, `seriesColor`, and `labelExpression` are recognized or warned and do not change the plot yet. The colors are the Chart.js 4.4.6 defaults. The face is Liberation Sans. See [Charts](charts.md).

## Unsupported tags

A tag in none of the tables in this document is unsupported. The render continues. The warning names the tag, and the tag is left off the page, except for a chart tag: any element whose name ends in `Chart` and is not one of the five supported kinds keeps the gray placeholder labeled Chart and warns `chart`.

### Report

| Tag | Result |
| --- | --- |
| `scriptlet` | Not run. |
| `parameter` | Not read. `$P{…}` in an expression warns on its own. |
| `sortField` | The caller’s document order is kept. |
| `variable` | Not read. `$V{PAGE_NUMBER}` is the only variable expression that fills. |
| `filterExpression` | Every record is kept. |
| `lastPageFooter` | The band is skipped. `pageFooter` is used on every page. |
| `noData` | The band is skipped. |
| `groupHeader`, `groupFooter` | The band is skipped. A `group` stored on a `subDataset` is ignored instead. |

### Band elements

| Tag | Result |
| --- | --- |
| `frame` | Omitted, including the elements inside it. |
| `crosstab` | Omitted. |
| `genericElement` | Omitted. |
| `printWhenExpression` | The element is still drawn. |
| `connectionExpression` | The dataset run is kept. Only `subDataSource("…")` supplies rows. |

### Components other than `jr:table`

| Tag | Result |
| --- | --- |
| `jr:list` | Omitted. |
| `jr:map` | Omitted. |
| Barcode components | Omitted. The samples use Barbecue and Barcode4j component names. |

### Chart tags

These keep the gray placeholder and warn `chart`:

`pieChart`, `pie3DChart`, `bar3DChart`, `stackedBar3DChart`, `xyBarChart`, `xyLineChart`, `xyAreaChart`, `scatterChart`, `bubbleChart`, `timeSeriesChart`, `highLowChart`, `candlestickChart`, `meterChart`, `thermometerChart`, `ganttChart`, `spiderChart`, `multiAxisChart`.

## How a gap is reported

Each unsupported tag or expression becomes a warning on the render result. The warning carries the tag or expression text and enough of its location to find it in the file. The rest of the document still renders. The viewer shows that list next to the PDF.

## Behaviors the samples require

These are not extra tags. They are fill and layout rules the files in `jrxmls/` already assume, taken from the JasperReports 6.20.1 model. The engine implements them.

| Behavior | Rule in this subset |
| --- | --- |
| No `.jasper` compile step | JRXML is interpreted. Expressions are not compiled to Java bytecode. |
| One main record | A JSON object is one record for the title, detail, and page bands. The engine does not iterate a query result for the main report. |
| Default font | Samples set `size` and `isBold` and do not set `fontName`. JasperReports defaults PDF text to Helvetica; pdfrost embeds Liberation Sans (regular, bold, italic, and bold italic), a Helvetica-metric stand-in. `Helvetica`, `SansSerif`, `Arial`, `Liberation Sans`, and `DejaVu Sans` use that face. Any other `fontName` is kept, warned, and still drawn with Liberation Sans. |
| Clip text | Text wraps inside the element box. A line break starts a new line. `Justified` spreads the words on a full line, and the last line of a paragraph stays left. Lines that do not fit the `reportElement` height are clipped. `isStretchWithOverflow="true"` grows that box to the text and grows the band. An element with `positionType="Float"` or `FixRelativeToBottom` moves down when an element above it stretches. |
| Missing values | A missing field or a null JSON value prints blank and adds a warning. JasperReports would print the word `null` because these templates leave `isBlankWhenNull` unset. pdfrost does not. Comparisons with JasperReports must allow that difference. |
| Numbers | A field with `class="java.lang.Integer"` prints a JSON number or a numeric string as digits. The samples set no `pattern`. |
| Timestamp | `new java.util.Date()` prints `YYYY-MM-DD HH:MM:SS` in UTC. The render call may pass the instant so tests stay fixed. JasperReports formats the date with the fill locale. That locale match is out of scope. |
| `splitType="Stretch"` | Every band in the samples uses `Stretch`. Layout does not start the band unless the space left on the page is at least the band's declared height. The extra height of a stretched band may continue on the next page. Any other `splitType` is a warning, and the band is treated as `Stretch`. |
| Pens and alignment | `lineWidth`, `lineColor`, and `lineStyle="Solid"` are drawn. A missing `lineWidth` is 0 and is not stroked. `textAlignment` and `verticalAlignment` apply to each wrapped line. Box padding and paragraph `leftIndent` and `rightIndent` inset the line. `firstLineIndent` insets the first line of each paragraph. `spacingBefore` is added above the first line. A tab advances to the next `tabStop`, or to the next multiple of `tabStopWidth` (40 when unset). `isItalic` uses the italic face, `isUnderline` draws a line under each run, and `isStrikeThrough` draws a line through it. Line spacing uses the font's ascent, descent, and leading. The first line stays single. Later lines follow `Single`, `1_1_2`, `Double`, `Proportional`, `AtLeast`, or `Fixed`. `rotation` of `Left`, `Right`, or `UpsideDown` turns the text inside the box. `markup="styled"` and `markup="html"` read `<style>`, `<b>`, `<i>`, `<u>`, `<font>`, `<s>`, `<sup>`, `<sub>`, and `<br>`. |
| Named table styles | A table section's `style` attribute selects a report `style` by exact name, including `Table 1_CH` in `Receive.jrxml`. |
| Empty table, headers kept | `jr:table` `whenNoDataType="AllSectionsNoDetail"` (`StockAdjustment.jrxml`, `Receive.jrxml`) still draws the header sections when the row array is missing or empty. Detail cells are omitted. |
| Empty column footer | A `jr:columnFooter` with a height and no children still occupies that height. `Receive.jrxml` uses 30. |

## Left outside the subset

Documented JasperReports behavior that these templates do not use. It stays out until a template depends on it.

| Area | Why it stays out |
| --- | --- |
| Other exporters | HTML, Excel, Word, and CSV. PDF is the only output. |
| Data-source adapters | JDBC, CSV, XML, JavaBeans, and executing `queryString`. The caller passes JSON. |
| General expressions | A Java or Groovy compiler, `$P{…}`, `$R{…}`, and Java calls other than `subDataSource("…")`. |
| Page X of Y | A second `$V{PAGE_NUMBER}` with `evaluationTime="Report"`. No text field in the samples sets `evaluationTime`. A chart may set `evaluationTime`, and `Now` and `Report` fill the same way. |
| Stretch with overflow | `isStretchWithOverflow` grows a text field in a title, detail, or summary band. A stretch inside a table cell does not grow the row. |
| PDF document features | Encryption, PDF/A, metadata, bookmarks, and compression flags. |
| Report-level empty data | `whenNoDataType` on `jasperReport` (`NoPages`, `BlankPage`, `NoData`). No sample sets it. |
| Extra pagination | Multiple columns, `isTitleNewPage`, `isSummaryNewPage`, and `lastPageFooter`. |
