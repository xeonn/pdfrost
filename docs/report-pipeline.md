# Report pipeline

The engine turns a template and a JSON document into PDF bytes in four steps. The caller supplies both inputs. The engine does not query a database and does not open a window.

JasperReports measures JRXML in pixels at 72 per inch, which is the PDF point. pdfrost uses that same unit. Element positions inside a band are from the band's top-left. PDF drawing is from the page's bottom-left, so layout converts.

## 1. Parse

Read the `.jrxml` XML into a report model.

The model holds the report name, page width and height, margins, styles, fields, subdatasets, bands, and the elements in each band. Supported tags become typed values. Parsed-and-ignored tags are dropped after they are recognized. Every other tag is recorded on the warning list with its name.

Field definitions keep `net.sf.jasperreports.json.field.expression` when it is present. That path is how a later fill reads `audit.created` or `product.id` from the JSON document. A field with no such property is read by its field name.

Parse fails when the XML is not a `jasperReport` document. It does not fail because a chart or another unsupported tag is present.

## 2. Fill

Walk the model and bind it to the JSON document.

The document is one JSON object, and that object is the single main record. Title, detail, and the page bands fill from it once. The engine does not walk a list of root records and does not execute `queryString`. Text fields outside a table read `$F{name}` from that object, using the field's JSON path when the template defines one. A field whose `class` is `java.lang.Integer` prints a JSON number or a numeric string as digits. The samples declare no `pattern`.

A table's `dataSourceExpression` must be a `subDataSource("…")` call. The string is a property of the same document whose value is an array. The operational templates all use `itemsList`. Each object in that array is one detail row. Fields declared inside that `subDataset` stay on the dataset. They are not aliases of the main report fields. The table reads `$F{…}` from the row.

Expressions that fill a value:

| Expression | Value |
| --- | --- |
| `$F{name}` | The field on the current record, report or table row. |
| `$V{PAGE_NUMBER}` | The page number assigned during layout. Fill records that the expression is the page number. Layout writes the digits. |
| `new java.util.Date()` | The render time, printed as `YYYY-MM-DD HH:MM:SS` in UTC. The caller may supply the instant. This is a fixed form, not a Java evaluator, and it does not follow the JasperReports fill locale. |

Any other expression text produces a warning and an empty value. A missing field or a null JSON value also produces an empty value and a warning that names the field. The word `null` is not printed. JasperReports would print that word for these templates, because they leave `isBlankWhenNull` unset.

`queryString` is not executed. Groups stored on a `subDataset` do not create bands.

## 3. Layout

Place filled elements on pages.

Page content width is `pageWidth - leftMargin - rightMargin`. Content height is `pageHeight - topMargin - bottomMargin`.

Band order follows JasperReports for the bands this subset supports:

1. `title` on the first page.
2. `pageHeader` on every page.
3. `columnHeader` where the template defines it.
4. `detail`, including a table that grows with its rows.
5. `summary` after the detail.
6. `columnFooter` above the page footer on every page. Its height stays clear of the bands above.
7. `pageFooter` on every page.
8. `background` behind the others, when the band has elements.

Every sample band sets `splitType="Stretch"`. Layout does not start that band unless the space remaining on the page is at least the band's declared height. A table row or a detail band that does not fit moves to the next page. The page header, the page footer, and the table's `jr:columnHeader` repeat on the new page. `$V{PAGE_NUMBER}` is the number of that page. There is no "Page X of Y": no text field sets `evaluationTime="Report"`.

Elements on a band paint in document order. A rectangle listed before a label stays behind that label. An `elementGroup` is flattened onto that band, so its children keep their own coordinates. An `ellipse` is drawn in its element box. An `image` draws a PNG or JPEG from a quoted path or from `$F{name}` when that field is a path. `RealHeight` and `RealSize` can grow the band. The picture is loaded while the page is drawn, including when `evaluationTime` is not `Now`. A `break` of type `Page` or `Column` in the title, detail, or summary opens the next page, and the elements after it in that band are placed from the new page.

A `jr:table` with `whenNoDataType="AllSectionsNoDetail"` still lays out its header sections when `itemsList` is missing or empty, and lays out no detail cells. A `jr:columnFooter` with a height and no children still consumes that height.

Static text, lines, and rectangles take the position on their `reportElement`. Horizontal and vertical alignment, bold, italic, underline, strike-through, font size, line spacing, and paragraph indents come from `textElement`, `font`, and `paragraph`. The samples do not set `fontName`. JasperReports would use Helvetica for PDF; pdfrost embeds Liberation Sans so the page does not depend on fonts on the machine. Text that does not fit the element box is clipped. `isStretchWithOverflow` grows the box in a flowing band. Borders use `lineWidth`, `lineColor`, and solid `lineStyle`, from `box` or from the named `style` on a table section. A missing `lineWidth` is 0. A width of 0 is not stroked.

## 4. Export

Draw the laid-out pages to a PDF. PDF is the only output. There is no `.jasper` compile step before this. The bytes are the return value, together with the warnings collected in the earlier steps. The embedded sans is subset into the file so the page does not depend on fonts installed on the machine. Encryption, PDF/A, document metadata, and bookmarks are not written.

Text, rules, rectangles, ellipses, table cells, and category charts are drawn. An unsupported element is absent from the page and present in the warning list. `areaChart`, `barChart`, `lineChart`, `stackedAreaChart`, and `stackedBarChart` are painted by `pdfrost-charts` inside the element box, in Liberation Sans. Any other chart tag keeps a placeholder box, and the warning still names `chart`.

## Result

A render produces either:

- PDF bytes and a warning list, which may be empty, or
- an error and no PDF, when the template is not readable XML or not a `jasperReport`.

Suggested library shape, once the crate exists:

```rust
engine::render(template_xml, json_document) -> Result<RenderOutput, Error>

struct RenderOutput {
    pdf: Vec<u8>,
    warnings: Vec<Warning>,
}
```

`Warning` names the tag or expression and where it was found, so the viewer can show it and a test can assert it.
