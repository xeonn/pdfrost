# Designer plan

Build the Design tab so it reads like Jaspersoft Studio 6.20’s **Design** editor, for the tags in [JRXML subset](jrxml-subset.md). Preview stays the PDF from `render`. Source stays the XML editor that already exists.

JasperReports 6.20.1 is still the behavior reference. Studio 6.20.1.final is the screen reference: where the outline sits, how a band is drawn, and which properties a selected element shows. Studio’s query dialog and expression compiler stay out.

## Recommendation

Edit the engine’s `Report`. Save it with `write_report`. Draw the design sheet at the page size, with the bands inside the margins.

Three choices follow from that.

**The document is `Report`, already parsed and written by the engine.** The window loads that JSON with `load_report` and saves it with `save_report`, which calls `write_report`. Warnings travel with the JSON and stay read-only. The old `Design` type, which dropped every non-table element into the title band, is gone.

**The canvas is Studio’s band editor, sized to the paper.** Studio’s design height is `topMargin` plus every band height plus `bottomMargin`. The sheet is `pageWidth` wide. Element `x` is measured from the left margin, so the white grid is only the column width (`pageWidth - leftMargin - rightMargin`); the margins are shaded and are not part of that coordinate. This editor uses that same sheet. It is at least `pageHeight` tall, so A4, Letter, and a custom size are the canvas. When the bands fit, the unused space stays inside the bottom margin. When they run past the page, the sheet grows and a dashed line marks `pageHeight`. Preview is still where the filled page appears.

**Ship the editor in slices that each leave a usable Design tab.** Load and show the open template first. Then select an element and edit its properties. Then place, move, and resize. Then fields, expressions, and one table. Then a new blank report. Undo and design zoom follow placement.

Opening a Studio file and saving it is lossy. `write_report` emits the supported subset, including a category chart's series. `com.jaspersoft.studio.*` properties and unsupported tags are omitted. The first save of a template whose parse warnings are non-empty asks for confirmation and names those tags. After a clean save, Preview uses the same `render_report` path as today.

## Goal

1. Open a `.jrxml` the engine already parses and show its bands and elements on the Design tab.
2. Change page size, margins, static text, text fields, images, lines, rectangles, and a simple table.
3. Save a `.jrxml` that `parse` accepts and that Preview renders without hand-written XML.
4. Keep Design, Source, and Preview as three views of one file, the way Studio’s editor tabs do.

## Reference

Studio’s report editor (6.20, Report Design) has three tabs: **Design**, **Source**, **Preview**. This app already has those tabs. Design is the one this plan fills in.

On Design, Studio arranges the work like this:

- **Outline** (report inspector), usually on the left. The report node, then bands in order, then the elements inside each band. A **Fields** node lists `$F{…}` names. Selecting a node selects it on the canvas. The header hides the panel, the remaining rail shows it again, and dragging its inner edge resizes it.
- **Band canvas** in the center. The sheet is the selected page size. A horizontal ruler and a vertical ruler stay pinned to the viewport and aligned with the page: a tick every 10 points, a label every 100, the margin ranges tinted, and the selected band or element marked. Margins on the sheet are shaded. Bands stack in the column between the left and right margins. Each band name is a light watermark in the column. Click empty space in a band, its margins, or its outline row to select it. The properties then show its height and whether it fits on the page. A thin line divides the bands, and dragging that line changes the band height. A selected band is tinted. Elements are boxes in document order, later elements painted above earlier ones. Static text shows its text. A text field shows its expression (`$F{docno}`), not a filled value. Both wrap, keep line breaks, and use the element's alignment, indents, first-line indent, spacing before, tabs, padding, size, bold, italic, underline, strike-through, line spacing, rotation, and styled or HTML markup. A table cell does the same, on top of that section's style fill and border. Stretch and `Float` are applied in the PDF; Design keeps the template boxes. Selection draws handles. Arrow keys nudge. Delete removes the element.
- **Palette** of tools, with **Properties** below it, on the right. The ones this subset can emit: Static Text, Text Field, Image, Line, Rectangle, Chart, Subreport, Table. An image drops at 50×50. Its expression is a quoted path or a field. A chart drops at 200×200 as a bar chart, with evaluation time Report, the legend on, and one series (`"SERIES 1"`, category `0`, value `0`). A subreport drops at the column width by 40. Its expression is a quoted `.jrxml` path. An empty data path fills that file once from the current JSON object. Properties can open the JSON document and pick an array or object; that path becomes `subDataSource`. Studio’s Frame, Ellipse, Crosstab, and barcode stay off the palette. The header hides both, the remaining rail shows them again, and dragging the inner edge resizes the panel.
- **Properties** for the current selection. For an element: `x`, `y`, `width`, `height`, and the text or expression. For an image: the image expression, scale, alignment, error behavior, lazy, cache, evaluation time, and rotation. For a chart: kind, title, subtitle, legend, evaluation time, the JSON array that supplies points, and each series name, category, and value. For text: size, bold, italic, underline, strike-through, markup, stretch, horizontal alignment, vertical alignment, line spacing, rotation, first-line indent, space before, and tab width. Every element has a border width and color. The width defaults to 0 and the PDF does not stroke it. Design still draws a light guide around that element, and a real border replaces the guide once the width is greater than 0. A rectangle is the box, not a label. For the report node: name, page width, page height, four margins, with A4 (595×842) and Letter (612×792) as presets. Column width is computed from the page width and the left and right margins.
- **Grid.** Studio snaps to a grid. Use a 10-point grid. Arrow keys move by 1 point. Shift-arrow moves by 10.

Studio interactions to copy, in order:

1. Click an element or its outline row to select it.
2. Edit the selection in Properties. Studio uses a properties view, not a modal, for these fields.
3. Choose a palette tool, then click a band to drop a default-size element (Static Text 100×20, Text Field 100×16, Image 50×50, Line 80×1, Rectangle 40×20, Chart 200×200, Subreport the column width by 40). Dragging a tool from the palette onto a band does the same. Every band in this subset is optional. The canvas and outline show only the bands the report has, top to bottom: Title, Page Header, Column Header, Detail, Column Footer, Page Footer, Summary, Background. Add a missing one from the outline. It starts at height 0, as a thin strip, and Delete removes it. The white space under the last band is unused page and does not take a drop.
4. Drag inside the band to move. Drag a handle to resize. Keep the element inside the band: clamp `x`/`y` to the column width and the band height.
5. Drag the band bar to set the band height. The height cannot shrink below the lowest element’s `y + height`.
6. Double-click static text to edit the string on the canvas. A text field’s expression stays in Properties, as a short field, with the known fields offered as `$F{name}`.

Studio pieces this plan leaves out: the data-adapter and query dialogs, the Java expression editor, parameters, variables, groups, styles as their own editor, align and distribute, guides, layers, and `lastPageFooter` / `noData` (those bands are outside the subset). A loaded `columnFooter` is shown with the other bands. Named styles already on a table section are kept on the model and written back. There is no style picker until a later pass.

## Layout

The window shell stays as it is: top bar, then the editor, then Report state. Design replaces the center when that tab is selected. Flowbite owns the chrome. The canvas is plain HTML positioned boxes. It is not a PDF and not a second layout engine.

```text
┌────────────┬──────────────────────────────────────┬────────────┐
│ Outline    │ Title                          79 ▾  │ Palette    │
│  Report    │  [ Company ]                         │ Static Text│
│  Fields    ├──────────────────────────────────────┤ Text Field │
│  Title     │ Page Header                    35 ▾  │ Line       │
│   · label  │                                      │ Rectangle  │
│  Detail    ├──────────────────────────────────────┤ Table      │
│   · table  │ Detail                        125 ▾  ├────────────┤
│  Page Foot │  [ $F{docno} ]                       │ Properties │
│            │                                      │ x y w h    │
│            │                                      │ text / $F  │
└────────────┴──────────────────────────────────────┴────────────┘
```

The right column is Palette above Properties. Studio docks those as separate views. One column matches the shell `Designer.vue` already has.

Outline rows:

| Node | Children |
| --- | --- |
| Report | The file name. Selecting it shows page setup. |
| Fields | One row per main-report field. Add and remove live here. |
| Each supported band | Title, Page Header, Column Header, Detail, Summary, Column Footer, Page Footer, Background, in that order when present. Elements under the band, in document order. A table opens further: its sections, then each column, then that column's cells and the text inside them. |
| Subdataset | Shown when the template has one, with its fields. A new table uses a dataset named `Items`. |

A band with no elements still appears, at its stored height, so a template can gain a title without hand-editing XML.

### Flowbite map

| Need | Piece |
| --- | --- |
| Outline and palette rows | Buttons, same pattern as the current Design shell |
| Properties fields | `FwbInput`, `FwbSelect`, `FwbToggle` (bold) |
| Page presets | `FwbSelect`: A4, Letter, Custom |
| Save / new | `FwbButton` on the Design toolbar |
| Lossy-save confirm | `window.confirm`, same as Source’s discard prompt |
| Unsupported tags | Existing Report state list. Design does not draw them. |

## Document

One file, three views.

| View | Holds | Writes |
| --- | --- | --- |
| Design | The `Report` JSON in memory | `write_report` to the template path |
| Source | The XML buffer it already keeps | The XML string, as it does today |
| Preview | PDF bytes from the last `render` | Nothing |

Switching into Design reloads from disk when Design is clean. If Source is dirty, ask to discard those edits first (the prompt Source already uses). Switching into Source from a dirty Design asks the same kind of question, then Source reads the file again after a save, or keeps the last saved XML after a discard.

Design does not fill fields. `$F{docno}` is the label on the box. Preview, with the chosen JSON or `{}`, is the filled page.

`columnWidth` is always `pageWidth - leftMargin - rightMargin`. The designer writes that value. It is not a separate field.

New expressions the user can attach to a text field:

| Choice | Written expression |
| --- | --- |
| A main-report field | `$F{name}` |
| Page number | `$V{PAGE_NUMBER}` |
| Printed at | `new java.util.Date()` |
| A table column | `$F{name}` on that column’s subdataset |

Any other expression already stored on a loaded template stays as text in Properties and still round-trips. The palette does not offer a way to invent a new unsupported expression.

A new table has `whenNoDataType="AllSectionsNoDetail"`, dataset `Items`, and `subDataSource("itemsList")`. Columns have a header label, a width, and a detail expression. Loaded tables keep their dataset name, data-source string, section heights, and style names. Select a section to edit its height and style, a column to edit its width, or the static text and text field inside a cell. Clicking the same parts on the canvas selects them too.

## Build order

1. **Load the open template into Design.** JSON for `Report`. Outline lists real bands and elements. The canvas is the page size. Each band is a box in the column, inside the shaded margins, with its text or expression. No edits. Opening `Sample.jrxml` shows the title, the column headings, the line, the rectangle, and a box for each supported chart. Preview plots those series when `chartPoints` is present.
2. **Select and edit.** Click a box or an outline row. Properties edit bounds, static text, the expression, font size, bold, and alignment. Save writes JRXML. Preview renders that file. Confirm when the loaded warnings would be dropped.
3. **Place, move, resize.** Drag a palette tool onto a band, or click the tool and then the band. Drag a field from the outline onto a band to create `$F{name}`. Drag to move, handles to resize, the divider to change height, Delete to remove, arrows to nudge. Clamp to the band.
4. **Fields and a table.** Add or remove a main-report field (name, `java.lang.String` or `java.lang.Integer`, JSON path). Bind a text field from that list. Drop one table on Detail and edit its columns.
5. **Page setup and a blank report.** Report properties: name, A4, Letter, custom size, four margins. **New** creates an empty report (A4, 20-point margins, empty title, detail, and page footer) and asks for a save path. Save As uses the same writer.
6. **Undo.** A short in-memory stack for property edits, moves, and deletes. The Design toolbar zooms the band stack from 50% to 200%, the same steps as Preview.

Each step leaves Design usable with the steps above it. Preview and Source keep working throughout.

## Done when

With `bun run tauri dev`:

1. Open `jrxmls/Sample.jrxml`. Design shows the title band and its static text, fields, line, and rectangle in the positions the file stores. The summary shows a box for each supported chart, labeled with its tag. Preview plots those series when `chartPoints` is present.
2. Change a label’s text and position, save, switch to Preview. The PDF shows the new label. Switch back to Design. The same box is still selected-capable and still shows the edit.
3. New report, drop a static text and a text field `$F{docno}`, save, choose a JSON object with `docno`. Preview prints the label and the value.
4. On that report, drop a one-column table bound to `itemsList`, save, and preview with a two-row array. Both rows appear.
5. Open `jrxmls/ShipmentOut.jrxml`. Bands and the line-item table appear. Saving after the confirm still parses. Preview still shows the document number and one line item when the fixture is chosen.
6. A dirty Source buffer cannot be overwritten by opening Design without a confirm.

## Out of this plan

- A palette tool that inserts an ellipse or a break. Loaded ellipses, breaks, and column footers stay on the canvas and round-trip. Frames, crosstabs, barcodes, parameters, variables, groups
- A style editor (existing style names on table sections are preserved, not authored)
- Stretch-with-overflow, print-when, evaluation time, Page X of Y
- Studio data adapters, the query dialog, and a Java expression editor
- Align, distribute, guides, and snap-to-element
- Project explorer and more than one open template
- Rewriting Source or the PDF stage

## Task list

- [x] JSON for `Report` and shell commands to load and save it through `parse` and `write_report`
- [x] Retire `Design`, `design_to_report`, and `save_template`
- [x] Replace the A4 sheet in `Designer.vue` with the band stack at column width
- [x] Outline of bands, elements, and fields from the loaded report
- [x] Selection and a Properties column for bounds, text, expression, font size, bold, alignment
- [x] Save, with a confirm when parse warnings would be omitted
- [x] Palette click-to-place, drag move, resize handles, band-height drag, Delete, arrow nudge
- [x] Main-report fields and a detail-band table with editable columns
- [x] Page setup (A4, Letter, custom, margins) and New / Save As
- [x] Dirty-state confirm when leaving Source for Design or Design for Source
- [x] In-memory undo for edits on the Design tab
- [ ] Done when: Sample opens on the band canvas and Preview plots the five category charts
- [ ] Done when: an edited label saves and shows on Preview
- [ ] Done when: a new report with a label, a `$F{docno}` field, and a one-column table previews from the app alone
- [ ] Done when: ShipmentOut opens, saves after confirm, and still previews a document number and a line item
