/** JSON shape of the engine `Report`. Field names match serde camelCase. */

export type BandKind =
  | "title"
  | "pageHeader"
  | "columnHeader"
  | "detail"
  | "summary"
  | "columnFooter"
  | "pageFooter"
  | "background";

export type SectionKind =
  | "tableHeader"
  | "columnHeader"
  | "detailCell"
  | "columnFooter"
  | "tableFooter";

export const SECTION_ORDER: SectionKind[] = [
  "tableHeader",
  "columnHeader",
  "detailCell",
  "columnFooter",
  "tableFooter",
];

export const SECTION_LABEL: Record<SectionKind, string> = {
  tableHeader: "Table Header",
  columnHeader: "Column Header",
  detailCell: "Detail",
  columnFooter: "Column Footer",
  tableFooter: "Table Footer",
};

export type Color = { r: number; g: number; b: number };

export type Pen = {
  lineWidth: number | null;
  lineColor: Color | null;
  lineStyle: string | null;
};

export type Borders = {
  pen: Pen | null;
  top: Pen | null;
  left: Pen | null;
  bottom: Pen | null;
  right: Pen | null;
  paddingTop?: number;
  paddingLeft?: number;
  paddingBottom?: number;
  paddingRight?: number;
};

export type ReportElement = {
  x: number;
  y: number;
  width: number;
  height: number;
  mode: "opaque" | "transparent" | null;
  backcolor: Color | null;
  borders: Borders | null;
  position?: "fixRelativeToTop" | "float" | "fixRelativeToBottom";
};

export type TextStyle = {
  textAlignment: "left" | "center" | "right" | "justified" | null;
  verticalAlignment: "top" | "middle" | "bottom" | null;
  fontSize: number | null;
  bold: boolean;
  italic?: boolean;
  underline?: boolean;
  strikeThrough?: boolean;
  fontName?: string | null;
  lineSpacing?: "single" | "oneAndHalf" | "double" | "proportional" | "atLeast" | "fixed" | null;
  lineSpacingSize?: number | null;
  rotation?: "left" | "right" | "upsideDown" | null;
  markup?: "none" | "styled" | "html";
  leftIndent: number | null;
  rightIndent: number | null;
  firstLineIndent?: number | null;
  spacingBefore?: number | null;
  tabStopWidth?: number | null;
  tabStops?: { position: number; alignment: "left" | "center" | "right" }[];
};

export type Field = {
  name: string;
  className: string;
  description: string | null;
  jsonPath: string | null;
};

export type ColumnSection = {
  kind: SectionKind;
  style: string | null;
  height: number;
  elements: Element[];
};

export type Column = {
  width: number;
  sections: ColumnSection[];
};

export type CategorySeries = {
  seriesExpression: string;
  categoryExpression: string;
  valueExpression: string;
};

export type Element =
  | { type: "staticText"; element: ReportElement; textStyle: TextStyle; text: string; stretch?: boolean }
  | { type: "textField"; element: ReportElement; textStyle: TextStyle; expression: string; stretch?: boolean }
  | { type: "line"; element: ReportElement }
  | { type: "rectangle"; element: ReportElement }
  | { type: "ellipse"; element: ReportElement }
  | {
      type: "image";
      element: ReportElement;
      expression: string;
      expressionClass?: string | null;
      scaleImage?: "clip" | "fillFrame" | "retainShape" | "realHeight" | "realSize" | null;
      hAlign?: "left" | "center" | "right" | null;
      vAlign?: "top" | "middle" | "bottom" | null;
      rotation?: "left" | "right" | "upsideDown" | null;
      usingCache?: boolean | null;
      lazy?: boolean;
      onErrorType?: "error" | "blank" | "icon" | null;
      evaluationTime?: string | null;
      evaluationGroup?: string | null;
    }
  | { type: "break"; element: ReportElement; column: boolean }
  | {
      type: "chart";
      element: ReportElement;
      kind: string;
      evaluationTime?: string | null;
      title?: string | null;
      subtitle?: string | null;
      showLegend?: boolean;
      dataSource?: string | null;
      subDataset?: string | null;
      series?: CategorySeries[];
    }
  | {
      type: "subreport";
      element: ReportElement;
      expression: string;
      dataSource?: string | null;
    }
  | {
      type: "table";
      element: ReportElement;
      whenNoData: string | null;
      subDataset: string | null;
      dataSource: string | null;
      columns: Column[];
    };

export type Band = {
  kind: BandKind;
  height: number;
  splitType: "stretch";
  elements: Element[];
};

export type Style = {
  name: string;
  mode: "opaque" | "transparent" | null;
  backcolor: Color | null;
  borders: Borders | null;
};

export type SubDataset = {
  name: string;
  fields: Field[];
};

export type Warning = {
  tag: string;
  location: string;
};

export type Report = {
  name: string;
  pageWidth: number;
  pageHeight: number;
  columnWidth: number;
  leftMargin: number;
  rightMargin: number;
  topMargin: number;
  bottomMargin: number;
  styles: Style[];
  fields: Field[];
  subdatasets: SubDataset[];
  bands: Band[];
  warnings: Warning[];
};

/** JRXML element order. A new band is inserted here. */
export const BAND_ORDER: BandKind[] = [
  "background",
  "title",
  "pageHeader",
  "columnHeader",
  "detail",
  "columnFooter",
  "pageFooter",
  "summary",
];

/** Design canvas, top to bottom. Background is last, as in Jaspersoft Studio. */
export const BAND_CANVAS: BandKind[] = [
  "title",
  "pageHeader",
  "columnHeader",
  "detail",
  "columnFooter",
  "pageFooter",
  "summary",
  "background",
];

export const BAND_LABEL: Record<BandKind, string> = {
  title: "Title",
  pageHeader: "Page Header",
  columnHeader: "Column Header",
  detail: "Detail",
  summary: "Summary",
  columnFooter: "Column Footer",
  pageFooter: "Page Footer",
  background: "Background",
};

export type Tool = "staticText" | "textField" | "image" | "line" | "rectangle" | "table" | "chart" | "subreport";

export const CHART_KINDS: { id: string; label: string }[] = [
  { id: "barChart", label: "Bar" },
  { id: "stackedBarChart", label: "Stacked bar" },
  { id: "lineChart", label: "Line" },
  { id: "areaChart", label: "Area" },
  { id: "stackedAreaChart", label: "Stacked area" },
];

export type Handle = "n" | "s" | "e" | "w" | "ne" | "nw" | "se" | "sw";

const PAGE_NUMBER = "$V{PAGE_NUMBER}";
const PRINTED_AT = "new java.util.Date()";

export function blankReport(): Report {
  return {
    name: "Report",
    pageWidth: 595,
    pageHeight: 842,
    columnWidth: 555,
    leftMargin: 20,
    rightMargin: 20,
    topMargin: 20,
    bottomMargin: 20,
    styles: [],
    fields: [],
    subdatasets: [],
    bands: [emptyBand("title", 79), emptyBand("detail", 125), emptyBand("pageFooter", 54)],
    warnings: [],
  };
}

export function syncColumn(report: Report): void {
  report.columnWidth = Math.max(0, report.pageWidth - report.leftMargin - report.rightMargin);
}

export function contentBottom(band: Band): number {
  return band.elements.reduce(
    (max, element) => Math.max(max, element.element.y + element.element.height),
    0,
  );
}

const PLOTTED_CHARTS = new Set([
  "areaChart",
  "barChart",
  "lineChart",
  "stackedAreaChart",
  "stackedBarChart",
]);

export function plotsChart(kind: string): boolean {
  return PLOTTED_CHARTS.has(kind);
}

export function elementLabel(element: Element): string {
  switch (element.type) {
    case "staticText":
      return element.text || "Static Text";
    case "textField":
      return element.expression || "Text Field";
    case "line":
      return "Line";
    case "rectangle":
      return "Rectangle";
    case "ellipse":
      return "Ellipse";
    case "image":
      return element.expression.replace(/^"|"$/g, "") || "Image";
    case "break":
      return "Break";
    case "chart":
      return element.kind || "Chart";
    case "subreport":
      return element.expression.replace(/^"|"$/g, "") || "Subreport";
    case "table":
      return "Table";
  }
}

export function snap(value: number): number {
  return Math.round(value / 10) * 10;
}

export function clampElement(element: ReportElement, columnWidth: number, bandHeight: number): void {
  const maxWidth = Math.max(1, columnWidth);
  const maxHeight = Math.max(1, bandHeight);
  element.width = Math.min(Math.max(1, element.width), maxWidth);
  element.height = Math.min(Math.max(1, element.height), maxHeight);
  element.x = Math.min(Math.max(0, element.x), Math.max(0, columnWidth - element.width));
  element.y = Math.min(Math.max(0, element.y), Math.max(0, bandHeight - element.height));
}

export function placeField(report: Report, bandIndex: number, name: string, x: number, y: number): number | null {
  const trimmed = name.trim();
  if (!trimmed) {
    return null;
  }
  return placePrepared(report, bandIndex, textField(snap(x), snap(y), 100, 16, fieldExpression(trimmed)));
}

export function placeElement(report: Report, bandIndex: number, tool: Tool, x: number, y: number): number | null {
  const band = report.bands[bandIndex];
  if (!band) {
    return null;
  }
  if (tool === "table" && band.kind !== "detail") {
    return null;
  }
  return placePrepared(report, bandIndex, createElement(report, tool, snap(x), snap(y)));
}

function placePrepared(report: Report, bandIndex: number, element: Element): number | null {
  const band = report.bands[bandIndex];
  if (!band) {
    return null;
  }
  const bottom = element.element.y + element.element.height;
  if (band.height < bottom) {
    band.height = bottom;
  }
  clampElement(element.element, report.columnWidth, band.height);
  band.elements.push(element);
  return band.elements.length - 1;
}

export function nudge(element: ReportElement, dx: number, dy: number, columnWidth: number, bandHeight: number): void {
  element.x += dx;
  element.y += dy;
  clampElement(element, columnWidth, bandHeight);
}

export function applyResize(
  element: ReportElement,
  handle: Handle,
  dx: number,
  dy: number,
  columnWidth: number,
  bandHeight: number,
): void {
  const start = { ...element };
  const east = handle === "e" || handle === "ne" || handle === "se";
  const west = handle === "w" || handle === "nw" || handle === "sw";
  const north = handle === "n" || handle === "ne" || handle === "nw";
  const south = handle === "s" || handle === "se" || handle === "sw";
  let x = start.x;
  let y = start.y;
  let width = start.width;
  let height = start.height;
  const right = start.x + start.width;
  const bottom = start.y + start.height;
  if (east) {
    width = snap(start.x + start.width + dx) - start.x;
  }
  if (south) {
    height = snap(start.y + start.height + dy) - start.y;
  }
  if (west) {
    x = snap(start.x + dx);
    width = right - x;
  }
  if (north) {
    y = snap(start.y + dy);
    height = bottom - y;
  }
  element.x = x;
  element.y = y;
  element.width = Math.max(1, width);
  element.height = Math.max(1, height);
  clampElement(element, columnWidth, bandHeight);
}

export function fieldExpression(name: string): string {
  return `$F{${name}}`;
}

export function chartExpression(value: string): string {
  const trimmed = value.trim();
  if (!trimmed || trimmed.startsWith("$") || trimmed.startsWith('"') || /^-?\d+(?:\.\d+)?$/.test(trimmed)) {
    return trimmed;
  }
  return `"${trimmed.replace(/"/g, '\\"')}"`;
}

export function chartChoices(
  report: Report,
  chart: Extract<Element, { type: "chart" }>,
  current: string,
): string[] {
  const dataset = chart.subDataset
    ? report.subdatasets.find((item) => item.name === chart.subDataset)
    : null;
  return expressionChoices(dataset ? dataset.fields : report.fields, current, false);
}

export function bindChartSource(report: Report, chart: Extract<Element, { type: "chart" }>, source: string): void {
  const name = source.trim();
  if (!name) {
    chart.dataSource = null;
    chart.subDataset = null;
    return;
  }
  chart.dataSource = name;
  chart.subDataset = ensureDataset(report, name).name;
  rememberChartFields(report, chart);
}

export function rememberChartFields(report: Report, chart: Extract<Element, { type: "chart" }>): void {
  const datasetName = chart.subDataset;
  if (!datasetName) {
    return;
  }
  const dataset = ensureDataset(report, datasetName);
  for (const series of chart.series ?? []) {
    for (const expression of [series.seriesExpression, series.categoryExpression, series.valueExpression]) {
      const name = fieldName(expression);
      if (name && !dataset.fields.some((field) => field.name === name)) {
        dataset.fields.push(makeField(name));
      }
    }
  }
}

function fieldName(expression: string): string | null {
  return /^\$F\{([A-Za-z_][A-Za-z0-9_]*)\}$/.exec(expression.trim())?.[1] ?? null;
}

export function expressionChoices(fields: Field[], current: string, includeBuiltins: boolean): string[] {
  const choices = fields.filter((field) => field.name).map((field) => fieldExpression(field.name));
  if (includeBuiltins) {
    choices.push(PAGE_NUMBER, PRINTED_AT);
  }
  if (current && !choices.includes(current)) {
    choices.unshift(current);
  }
  return choices;
}

export function columnHeaderText(column: Column): string {
  const section = column.sections.find((item) => item.kind === "columnHeader");
  const label = section?.elements.find((element) => element.type === "staticText");
  return label && label.type === "staticText" ? label.text : "";
}

export function setColumnHeader(column: Column, text: string): void {
  const section = ensureSection(column, "columnHeader", 30);
  let label = section.elements.find((element) => element.type === "staticText");
  if (!label || label.type !== "staticText") {
    label = staticText(0, 0, column.width, section.height || 30, text);
    section.elements.unshift(label);
    return;
  }
  label.text = text;
  label.element.width = column.width;
  label.element.height = section.height || label.element.height;
}

export function columnDetailExpression(column: Column): string {
  const section = column.sections.find((item) => item.kind === "detailCell");
  const field = section?.elements.find((element) => element.type === "textField");
  return field && field.type === "textField" ? field.expression : "";
}

export function setColumnDetail(column: Column, expression: string): void {
  const section = ensureSection(column, "detailCell", 30);
  let field = section.elements.find((element) => element.type === "textField");
  if (!field || field.type !== "textField") {
    field = textField(0, 0, column.width, section.height || 30, expression);
    section.elements.unshift(field);
    return;
  }
  field.expression = expression;
  field.element.width = column.width;
  field.element.height = section.height || field.element.height;
}

export function syncTableBox(table: Extract<Element, { type: "table" }>): void {
  table.element.width = table.columns.reduce((sum, column) => sum + column.width, 0);
  for (const column of table.columns) {
    for (const section of column.sections) {
      for (const element of section.elements) {
        element.element.width = column.width;
      }
    }
  }
  const minHeight = SECTION_ORDER.reduce(
    (sum, kind) => sum + kindHeight(table.columns, kind),
    0,
  );
  if (table.element.height < minHeight) {
    table.element.height = minHeight;
  }
}

export function tableSectionKinds(table: Extract<Element, { type: "table" }>): SectionKind[] {
  const present = new Set(table.columns.flatMap((column) => column.sections.map((section) => section.kind)));
  return SECTION_ORDER.filter((kind) => present.has(kind));
}

export function orderedSections(column: Column): { section: ColumnSection; index: number }[] {
  return SECTION_ORDER.flatMap((kind) => {
    const found = sectionByKind(column, kind);
    return found ? [found] : [];
  });
}

export function sectionByKind(column: Column, kind: SectionKind): { section: ColumnSection; index: number } | null {
  const index = column.sections.findIndex((section) => section.kind === kind);
  if (index < 0) {
    return null;
  }
  return { section: column.sections[index], index };
}

export function setSectionsHeight(table: Extract<Element, { type: "table" }>, kind: SectionKind, height: number): void {
  const next = Math.max(0, height);
  for (const column of table.columns) {
    const section = column.sections.find((item) => item.kind === kind);
    if (section) {
      section.height = next;
    }
  }
  syncTableBox(table);
}

export function setSectionsStyle(
  table: Extract<Element, { type: "table" }>,
  kind: SectionKind,
  style: string | null,
): void {
  for (const column of table.columns) {
    const section = column.sections.find((item) => item.kind === kind);
    if (section) {
      section.style = style;
    }
  }
}

export function addColumn(report: Report, table: Extract<Element, { type: "table" }>): void {
  const dataset = ensureDataset(report, table.subDataset || "Items");
  table.subDataset = dataset.name;
  if (!table.dataSource) {
    table.dataSource = "itemsList";
  }
  const name = uniqueName(
    dataset.fields.map((field) => field.name),
    "column",
  );
  dataset.fields.push(makeField(name));
  const source = table.columns[0];
  const column: Column = source
    ? (JSON.parse(JSON.stringify(source)) as Column)
    : freshColumn(100, "Column", fieldExpression(name));
  column.width = source ? source.width : 100;
  setColumnHeader(column, "Column");
  setColumnDetail(column, fieldExpression(name));
  table.columns.push(column);
  syncTableBox(table);
}

export function removeColumn(table: Extract<Element, { type: "table" }>, index: number): void {
  if (table.columns.length <= 1) {
    return;
  }
  table.columns.splice(index, 1);
  syncTableBox(table);
}

/** Read Fields keeps a description already stored for the same name. */
export function fieldsFromDiscovery(
  previous: Field[],
  found: { name: string; className: string; jsonPath: string }[],
): Field[] {
  const prior = new Map(previous.map((field) => [field.name, field]));
  return found.map((item) => ({
    name: item.name,
    className: item.className,
    description: prior.get(item.name)?.description ?? null,
    jsonPath: item.jsonPath,
  }));
}

export function addMainField(report: Report): number {
  const name = uniqueName(
    report.fields.map((field) => field.name),
    "field",
  );
  report.fields.push(makeField(name));
  return report.fields.length - 1;
}

export function addDatasetField(dataset: SubDataset): number {
  const name = uniqueName(
    dataset.fields.map((field) => field.name),
    "field",
  );
  dataset.fields.push(makeField(name));
  return dataset.fields.length - 1;
}

export function hexColor(color: Color | null): string {
  const value = color ?? { r: 0, g: 0, b: 0 };
  return `#${[value.r, value.g, value.b].map((part) => part.toString(16).padStart(2, "0")).join("")}`;
}

export function colorFromHex(hex: string): Color {
  const body = hex.replace("#", "");
  return {
    r: Number.parseInt(body.slice(0, 2), 16) || 0,
    g: Number.parseInt(body.slice(2, 4), 16) || 0,
    b: Number.parseInt(body.slice(4, 6), 16) || 0,
  };
}

export function borderWidth(element: ReportElement): number {
  const borders = element.borders;
  if (!borders) {
    return 0;
  }
  const pen = borders.pen ?? borders.top ?? borders.left ?? borders.bottom ?? borders.right;
  return pen?.lineWidth ?? 0;
}

export function borderColor(element: ReportElement): Color | null {
  const borders = element.borders;
  if (!borders) {
    return null;
  }
  return (
    borders.pen?.lineColor ??
    borders.top?.lineColor ??
    borders.left?.lineColor ??
    borders.bottom?.lineColor ??
    borders.right?.lineColor ??
    null
  );
}

export function ensurePen(element: ReportElement): Pen {
  if (!element.borders) {
    element.borders = { pen: null, top: null, left: null, bottom: null, right: null };
  }
  if (!element.borders.pen) {
    element.borders.pen = { lineWidth: 0, lineColor: { r: 0, g: 0, b: 0 }, lineStyle: "Solid" };
  }
  return element.borders.pen;
}

export function setBorderWidth(element: ReportElement, width: number): void {
  const next = Math.max(0, width);
  if (next === 0 && !element.borders?.pen) {
    return;
  }
  ensurePen(element).lineWidth = next;
}

export function setBorderColor(element: ReportElement, color: Color): void {
  ensurePen(element).lineColor = color;
}

export function ensureBand(report: Report, kind: BandKind): number {
  const existing = report.bands.findIndex((band) => band.kind === kind);
  if (existing >= 0) {
    return existing;
  }
  const band = emptyBand(kind, 0);
  const order = BAND_ORDER.indexOf(kind);
  const insertAt = report.bands.findIndex((item) => BAND_ORDER.indexOf(item.kind) > order);
  if (insertAt < 0) {
    report.bands.push(band);
    return report.bands.length - 1;
  }
  report.bands.splice(insertAt, 0, band);
  return insertAt;
}

function emptyBand(kind: BandKind, height: number): Band {
  return { kind, height, splitType: "stretch", elements: [] };
}

function frame(x: number, y: number, width: number, height: number): ReportElement {
  return {
    x,
    y,
    width,
    height,
    mode: null,
    backcolor: null,
    borders: null,
  };
}

function textStyle(): TextStyle {
  return {
    textAlignment: null,
    verticalAlignment: null,
    fontSize: 10,
    bold: false,
    italic: false,
    underline: false,
    strikeThrough: false,
    fontName: null,
    lineSpacing: null,
    lineSpacingSize: null,
    rotation: null,
    markup: "none",
    leftIndent: null,
    rightIndent: null,
    firstLineIndent: null,
    spacingBefore: null,
    tabStopWidth: null,
  };
}

function staticText(
  x: number,
  y: number,
  width: number,
  height: number,
  text: string,
): Extract<Element, { type: "staticText" }> {
  return { type: "staticText", element: frame(x, y, width, height), textStyle: textStyle(), text };
}

function textField(
  x: number,
  y: number,
  width: number,
  height: number,
  expression: string,
): Extract<Element, { type: "textField" }> {
  return {
    type: "textField",
    element: frame(x, y, width, height),
    textStyle: textStyle(),
    expression,
  };
}

function createElement(report: Report, tool: Tool, x: number, y: number): Element {
  switch (tool) {
    case "staticText":
      return staticText(x, y, 100, 20, "Static Text");
    case "textField":
      return textField(x, y, 100, 16, report.fields[0] ? fieldExpression(report.fields[0].name) : PAGE_NUMBER);
    case "line":
      return { type: "line", element: frame(x, y, 80, 1) };
    case "rectangle":
      return { type: "rectangle", element: frame(x, y, 40, 20) };
    case "image":
      return { type: "image", element: frame(x, y, 50, 50), expression: "" };
    case "chart":
      return {
        type: "chart",
        element: frame(x, y, 200, 200),
        kind: "barChart",
        evaluationTime: "Report",
        showLegend: true,
        series: [
          { seriesExpression: '"SERIES 1"', categoryExpression: "0", valueExpression: "0" },
        ],
      };
    case "subreport":
      return {
        type: "subreport",
        element: frame(x, y, Math.max(40, report.columnWidth || 200), 40),
        expression: "",
        dataSource: null,
      };
    case "table":
      return newTable(report, x, y);
  }
}

function newTable(report: Report, x: number, y: number): Element {
  const dataset = ensureDataset(report, "Items");
  if (!dataset.fields.some((field) => field.name === "item")) {
    dataset.fields.push(makeField("item"));
  }
  const column = freshColumn(120, "Item", fieldExpression("item"));
  const table: Element = {
    type: "table",
    element: frame(x, y, 120, 60),
    whenNoData: "AllSectionsNoDetail",
    subDataset: dataset.name,
    dataSource: "itemsList",
    columns: [column],
  };
  syncTableBox(table);
  return table;
}

function freshColumn(width: number, header: string, expression: string): Column {
  const column: Column = { width, sections: [] };
  setColumnHeader(column, header);
  setColumnDetail(column, expression);
  return column;
}

function ensureDataset(report: Report, name: string): SubDataset {
  let dataset = report.subdatasets.find((item) => item.name === name);
  if (!dataset) {
    dataset = { name, fields: [] };
    report.subdatasets.push(dataset);
  }
  return dataset;
}

function ensureSection(column: Column, kind: SectionKind, height: number): ColumnSection {
  let section = column.sections.find((item) => item.kind === kind);
  if (!section) {
    section = { kind, style: null, height, elements: [] };
    column.sections.push(section);
  }
  return section;
}

function sectionHeight(column: Column | undefined, kind: SectionKind): number {
  return column?.sections.find((section) => section.kind === kind)?.height ?? 0;
}

function kindHeight(columns: Column[], kind: SectionKind): number {
  return columns.reduce((max, column) => Math.max(max, sectionHeight(column, kind)), 0);
}

function makeField(name: string): Field {
  return {
    name,
    className: "java.lang.String",
    description: null,
    jsonPath: name,
  };
}

function uniqueName(used: string[], base: string): string {
  if (!used.includes(base)) {
    return base;
  }
  let index = 2;
  while (used.includes(`${base}${index}`)) {
    index += 1;
  }
  return `${base}${index}`;
}
