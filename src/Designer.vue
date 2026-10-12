<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { FwbButton, FwbButtonGroup, FwbP } from "flowbite-vue";
import Icon from "./Icon.vue";
import JsonTree from "./JsonTree.vue";
import {
  BAND_LABEL,
  BAND_CANVAS,
  blankReport,
  CHART_KINDS,
  SECTION_LABEL,
  type SectionKind,
  addColumn,
  addDatasetField,
  addMainField,
  applyResize,
  bindChartSource,
  borderColor,
  borderWidth,
  chartChoices,
  chartExpression,
  clampElement,
  colorFromHex,
  contentBottom,
  elementLabel,
  ensureBand,
  expressionChoices,
  fieldsFromDiscovery,
  hexColor,
  nudge,
  orderedSections,
  placeElement,
  placeField,
  plotsChart,
  rememberChartFields,
  removeColumn,
  sectionByKind,
  setBorderColor,
  setBorderWidth,
  setSectionsHeight,
  setSectionsStyle,
  snap,
  syncColumn,
  syncTableBox,
  tableSectionKinds,
  type BandKind,
  type CategorySeries,
  type ColumnSection,
  type Element,
  type Field,
  type Handle,
  type Report,
  type ReportElement,
  type Tool,
  type Warning,
} from "./report";

const props = defineProps<{ templatePath: string; documentPath?: string; active?: boolean }>();
const emit = defineEmits<{
  saved: [path: string];
  openTemplate: [path: string];
  openDataset: [];
  datasets: [datasets: { id: string; name: string; fields: Field[] }[]];
}>();

type Selection =
  | { kind: "report" }
  | { kind: "fields" }
  | { kind: "field"; index: number }
  | { kind: "dataset"; index: number }
  | { kind: "datasetField"; dataset: number; index: number }
  | { kind: "band"; index: number }
  | { kind: "element"; band: number; index: number }
  | { kind: "tableSection"; band: number; index: number; section: SectionKind }
  | { kind: "column"; band: number; index: number; column: number }
  | { kind: "columnSection"; band: number; index: number; column: number; section: number }
  | { kind: "cell"; band: number; index: number; column: number; section: number; element: number };

type Drag =
  | { kind: "move"; band: number; index: number; originX: number; originY: number; pointerX: number; pointerY: number }
  | {
      kind: "resize";
      band: number;
      index: number;
      handle: Handle;
      originX: number;
      originY: number;
      originW: number;
      originH: number;
      pointerX: number;
      pointerY: number;
    }
  | { kind: "band"; band: number; origin: number; pointerY: number };

const HANDLES: Handle[] = ["nw", "n", "ne", "e", "se", "s", "sw", "w"];
const inputClass =
  "mt-1 w-full rounded-md border border-slate-700 bg-slate-900 px-2 py-1 text-sm text-gray-100 outline-none focus:border-blue-500";

const report = ref<Report | null>(null);
const selection = ref<Selection>({ kind: "report" });
const tool = ref<Tool | null>(null);
const outlineOpen = ref(true);
const inspectorOpen = ref(true);
const outlineWidth = ref(224);
const inspectorWidth = ref(288);
const dropBand = ref<number | null>(null);
const bandToAdd = ref<BandKind | "">("");
const EMPTY_STRIP = 18;
const DRAG_TOOL = "application/pdfrost-tool";
const DRAG_FIELD = "application/pdfrost-field";
const error = ref("");
const loading = ref(false);
const saving = ref(false);
const loadWarnings = ref<Warning[]>([]);
const lossyConfirmed = ref(true);
const confirmedPaths = new Set<string>();
const savedSnapshot = ref("");
const imagePreviews = ref<Record<string, string>>({});
let imagePreviewTicket = 0;
const undoStack = ref<string[]>([]);
const editing = ref<{ band: number; index: number } | null>(null);
const textDraft = ref("");
const palette: { id: Tool; label: string; icon: "type" | "field" | "image" | "minus" | "square" | "table" | "chart" | "subreport" }[] = [
  { id: "staticText", label: "Static Text", icon: "type" },
  { id: "textField", label: "Text Field", icon: "field" },
  { id: "image", label: "Image", icon: "image" },
  { id: "line", label: "Line", icon: "minus" },
  { id: "rectangle", label: "Rectangle", icon: "square" },
  { id: "chart", label: "Chart", icon: "chart" },
  { id: "subreport", label: "Subreport", icon: "subreport" },
  { id: "table", label: "Table", icon: "table" },
];
const jsonDocument = ref<unknown>(null);
const jsonError = ref("");
let generation = 0;
let drag: Drag | null = null;
let panelDrag: { side: "outline" | "inspector"; startX: number; startWidth: number } | null = null;

const dirty = computed(
  () => !!report.value && JSON.stringify(report.value) !== savedSnapshot.value,
);
const columnWidth = computed(() => report.value?.columnWidth ?? 0);
const leftMargin = computed(() => Math.max(0, report.value?.leftMargin ?? 0));
const rightMargin = computed(() => Math.max(0, report.value?.rightMargin ?? 0));
const topMargin = computed(() => Math.max(0, report.value?.topMargin ?? 0));
const bottomMargin = computed(() => Math.max(0, report.value?.bottomMargin ?? 0));
const canvasWidth = computed(() => {
  if (!report.value) {
    return 0;
  }
  return Math.max(report.value.pageWidth, leftMargin.value + report.value.columnWidth + rightMargin.value);
});
const bandRows = computed(() => {
  if (!report.value) {
    return [];
  }
  return BAND_CANVAS.flatMap((kind) => {
    const index = report.value!.bands.findIndex((band) => band.kind === kind);
    return index < 0 ? [] : [{ index, band: report.value!.bands[index] }];
  });
});
const designedHeight = computed(() => {
  const bands = bandRows.value.reduce((sum, row) => sum + stripHeight(row.band.height), 0);
  return topMargin.value + bands + bottomMargin.value;
});
const missingBands = computed(() => {
  if (!report.value) {
    return [];
  }
  const present = new Set(report.value.bands.map((band) => band.kind));
  return BAND_CANVAS.filter((kind) => !present.has(kind));
});
const sheetHeight = computed(() => {
  if (!report.value) {
    return 0;
  }
  return Math.max(report.value.pageHeight, designedHeight.value);
});
const pageRemainder = computed(() => Math.max(0, sheetHeight.value - designedHeight.value));
const pastPage = computed(() => !!report.value && designedHeight.value > report.value.pageHeight);
const selectedBand = computed(() => {
  if (!report.value || selection.value.kind !== "band") {
    return null;
  }
  return report.value.bands[selection.value.index] ?? null;
});
const bandMonitor = computed(() => {
  if (!report.value || !selectedBand.value || selection.value.kind !== "band") {
    return null;
  }
  let start = topMargin.value;
  for (const row of bandRows.value) {
    if (row.index === selection.value.index) {
      break;
    }
    start += stripHeight(row.band.height);
  }
  const end = start + stripHeight(selectedBand.value.height);
  const pageEdge = report.value.pageHeight;
  const contentLimit = Math.max(0, pageEdge - topMargin.value - bottomMargin.value);
  const stack = bandRows.value
    .filter((row) => row.band.kind !== "background")
    .reduce((sum, row) => sum + stripHeight(row.band.height), 0);
  return {
    start,
    end,
    pageEdge,
    contentLimit,
    stack,
    crosses: start < pageEdge && end > pageEdge,
    past: start >= pageEdge,
    stackOverflows: stack > contentLimit,
    floor: contentBottom(selectedBand.value),
  };
});
const selectedElement = computed(() => {
  if (!report.value || selection.value.kind !== "element") {
    return null;
  }
  return report.value.bands[selection.value.band]?.elements[selection.value.index] ?? null;
});
const selectedTable = computed(() => {
  if (!report.value) {
    return null;
  }
  const current = selection.value;
  if (
    current.kind !== "element" &&
    current.kind !== "tableSection" &&
    current.kind !== "column" &&
    current.kind !== "columnSection" &&
    current.kind !== "cell"
  ) {
    return null;
  }
  const element = report.value.bands[current.band]?.elements[current.index];
  return element?.type === "table" ? element : null;
});
const editedElement = computed(() => {
  if (selectedElement.value) {
    return selectedElement.value;
  }
  if (!selectedTable.value || selection.value.kind !== "cell") {
    return null;
  }
  const { column, section, element } = selection.value;
  return selectedTable.value.columns[column]?.sections[section]?.elements[element] ?? null;
});
const selectedColumn = computed(() => {
  if (!selectedTable.value || (selection.value.kind !== "column" && selection.value.kind !== "columnSection")) {
    return null;
  }
  return selectedTable.value.columns[selection.value.column] ?? null;
});
const selectedColumnSection = computed(() => {
  if (!selectedColumn.value || selection.value.kind !== "columnSection") {
    return null;
  }
  return selectedColumn.value.sections[selection.value.section] ?? null;
});
const selectedTableSection = computed(() => {
  if (!selectedTable.value || selection.value.kind !== "tableSection") {
    return null;
  }
  const kind = selection.value.section;
  return selectedTable.value.columns.flatMap((column) => column.sections).find((section) => section.kind === kind) ?? null;
});
const selectedField = computed(() => {
  if (!report.value) {
    return null;
  }
  if (selection.value.kind === "field") {
    return report.value.fields[selection.value.index] ?? null;
  }
  if (selection.value.kind === "datasetField") {
    return report.value.subdatasets[selection.value.dataset]?.fields[selection.value.index] ?? null;
  }
  return null;
});
const pagePreset = computed(() => {
  if (!report.value) {
    return "custom";
  }
  if (report.value.pageWidth === 595 && report.value.pageHeight === 842) {
    return "a4";
  }
  if (report.value.pageWidth === 612 && report.value.pageHeight === 792) {
    return "letter";
  }
  return "custom";
});

const ZOOM_STEPS = [0.5, 0.75, 1, 1.25, 1.5, 2];
const scale = ref(1);
const stageEl = ref<HTMLElement | null>(null);
const zoomOptions = ZOOM_STEPS.map((step) => ({
  name: `${Math.round(step * 100)}%`,
  value: String(step),
}));
const gridStyle = {
  backgroundImage:
    "linear-gradient(to right, rgba(148,163,184,0.12) 1px, transparent 1px), linear-gradient(to bottom, rgba(148,163,184,0.12) 1px, transparent 1px)",
  backgroundSize: "10px 10px",
};
const RULER = 24;
const STAGE_PAD = 24;
const hRulerView = ref<HTMLElement | null>(null);
const vRulerView = ref<HTMLElement | null>(null);
const hRuler = ref<HTMLCanvasElement | null>(null);
const vRuler = ref<HTMLCanvasElement | null>(null);
const sheetEl = ref<HTMLElement | null>(null);
const pointer = ref<{ x: number; y: number } | null>(null);

function bandTop(index: number): number {
  let y = topMargin.value;
  for (const row of bandRows.value) {
    if (row.index === index) {
      return y;
    }
    y += row.band.height;
  }
  return y;
}

function visualHeight(element: Element): number {
  if (element.type === "line" || element.type === "break") {
    return Math.max(element.element.height, 8);
  }
  return element.element.height;
}

const rulerSpan = computed(() => {
  if (!report.value) {
    return null;
  }
  const current = selection.value;
  if (current.kind === "element" && selectedElement.value) {
    return {
      x: leftMargin.value + selectedElement.value.element.x,
      y: bandTop(current.band) + selectedElement.value.element.y,
      width: selectedElement.value.element.width,
      height: visualHeight(selectedElement.value),
    };
  }
  if (current.kind === "band" && selectedBand.value) {
    return {
      x: leftMargin.value,
      y: bandTop(current.index),
      width: columnWidth.value,
      height: selectedBand.value.height,
    };
  }
  if (
    selectedTable.value &&
    (current.kind === "cell" ||
      current.kind === "column" ||
      current.kind === "columnSection" ||
      current.kind === "tableSection")
  ) {
    return {
      x: leftMargin.value + selectedTable.value.element.x,
      y: bandTop(current.band) + selectedTable.value.element.y,
      width: selectedTable.value.element.width,
      height: selectedTable.value.element.height,
    };
  }
  return null;
});

function labelStep(zoom: number): number {
  return [100, 200, 500, 1000].find((step) => step * zoom >= 40) ?? 1000;
}

function drawRuler(
  canvas: HTMLCanvasElement,
  along: number,
  vertical: boolean,
  marginStart: number,
  marginEnd: number,
): void {
  const zoom = scale.value;
  const dpr = window.devicePixelRatio || 1;
  const length = along * zoom;
  const thickness = RULER;
  canvas.style.width = `${vertical ? thickness : length}px`;
  canvas.style.height = `${vertical ? length : thickness}px`;
  canvas.width = Math.max(1, Math.round((vertical ? thickness : length) * dpr));
  canvas.height = Math.max(1, Math.round((vertical ? length : thickness) * dpr));
  const ctx = canvas.getContext("2d");
  if (!ctx) {
    return;
  }
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.fillStyle = "#1e293b";
  ctx.fillRect(0, 0, vertical ? thickness : length, vertical ? length : thickness);
  ctx.fillStyle = "rgba(148, 163, 184, 0.22)";
  if (vertical) {
    ctx.fillRect(0, 0, thickness, marginStart * zoom);
    ctx.fillRect(0, (along - marginEnd) * zoom, thickness, marginEnd * zoom);
  } else {
    ctx.fillRect(0, 0, marginStart * zoom, thickness);
    ctx.fillRect((along - marginEnd) * zoom, 0, marginEnd * zoom, thickness);
  }
  const span = rulerSpan.value;
  if (span) {
    ctx.fillStyle = "rgba(59, 130, 246, 0.45)";
    if (vertical) {
      ctx.fillRect(0, span.y * zoom, thickness, Math.max(span.height * zoom, 1));
    } else {
      ctx.fillRect(span.x * zoom, 0, Math.max(span.width * zoom, 1), thickness);
    }
  }
  ctx.strokeStyle = "#94a3b8";
  ctx.fillStyle = "#e2e8f0";
  ctx.lineWidth = 1;
  ctx.font = "9px sans-serif";
  ctx.textBaseline = "top";
  const labeled = labelStep(zoom);
  const minor = 10 * zoom >= 5;
  for (let unit = 0; unit <= along + 0.01; unit += 10) {
    const major = unit % labeled === 0;
    const mid = unit % 50 === 0;
    if (!major && !mid && !minor) {
      continue;
    }
    const at = unit * zoom;
    const tick = major ? 12 : mid ? 8 : 4;
    ctx.beginPath();
    if (vertical) {
      ctx.moveTo(thickness, at + 0.5);
      ctx.lineTo(thickness - tick, at + 0.5);
    } else {
      ctx.moveTo(at + 0.5, thickness);
      ctx.lineTo(at + 0.5, thickness - tick);
    }
    ctx.stroke();
    if (!major) {
      continue;
    }
    ctx.save();
    if (vertical) {
      ctx.translate(1, at + 11);
      ctx.rotate(-Math.PI / 2);
      ctx.textAlign = "left";
      ctx.fillText(String(unit), 0, 0);
    } else {
      ctx.textAlign = "left";
      ctx.fillText(String(unit), at + 2, 2);
    }
    ctx.restore();
  }
  if (report.value && pastPage.value) {
    ctx.strokeStyle = "#f59e0b";
    ctx.beginPath();
    const edge = report.value.pageHeight * zoom;
    if (vertical) {
      ctx.moveTo(0, edge + 0.5);
      ctx.lineTo(thickness, edge + 0.5);
    }
    ctx.stroke();
  }
  const mark = pointer.value;
  if (mark) {
    ctx.strokeStyle = "#fb923c";
    ctx.beginPath();
    if (vertical) {
      ctx.moveTo(0, mark.y * zoom + 0.5);
      ctx.lineTo(thickness, mark.y * zoom + 0.5);
    } else {
      ctx.moveTo(mark.x * zoom + 0.5, 0);
      ctx.lineTo(mark.x * zoom + 0.5, thickness);
    }
    ctx.stroke();
  }
}

function drawRulers(): void {
  if (!report.value || !hRuler.value || !vRuler.value) {
    return;
  }
  drawRuler(hRuler.value, canvasWidth.value, false, leftMargin.value, rightMargin.value);
  drawRuler(vRuler.value, sheetHeight.value, true, topMargin.value, bottomMargin.value);
  syncRulers();
}

function syncRulers(): void {
  const stage = stageEl.value;
  if (!stage) {
    return;
  }
  if (hRulerView.value) {
    hRulerView.value.style.marginRight = `${stage.offsetWidth - stage.clientWidth}px`;
    hRulerView.value.scrollLeft = stage.scrollLeft;
  }
  if (vRulerView.value) {
    vRulerView.value.style.marginBottom = `${stage.offsetHeight - stage.clientHeight}px`;
    vRulerView.value.scrollTop = stage.scrollTop;
  }
}

function pagePoint(event: PointerEvent): { x: number; y: number } | null {
  const rect = sheetEl.value?.getBoundingClientRect();
  if (!rect || scale.value <= 0) {
    return null;
  }
  const x = (event.clientX - rect.left) / scale.value;
  const y = (event.clientY - rect.top) / scale.value;
  if (x < 0 || y < 0 || x > canvasWidth.value || y > sheetHeight.value) {
    return null;
  }
  return { x, y };
}

function trackPointer(event: PointerEvent): void {
  pointer.value = pagePoint(event);
}

function onRulerWheel(event: WheelEvent): void {
  if (event.ctrlKey || event.metaKey) {
    onWheel(event);
    return;
  }
  stageEl.value?.scrollBy({ left: event.deltaX, top: event.deltaY });
}

function zoomBy(direction: -1 | 1): void {
  if (direction < 0) {
    const index = ZOOM_STEPS.findIndex((step) => step >= scale.value - 0.001);
    const current = index === -1 ? ZOOM_STEPS.length - 1 : index;
    scale.value = ZOOM_STEPS[Math.max(0, current - 1)] ?? 0.5;
    return;
  }
  const index = ZOOM_STEPS.findIndex((step) => step > scale.value + 0.001);
  scale.value = ZOOM_STEPS[index === -1 ? ZOOM_STEPS.length - 1 : index] ?? 2;
}

function setZoom(raw: string): void {
  const next = Number(raw);
  if (!Number.isFinite(next) || next <= 0) {
    return;
  }
  scale.value = next;
}

function onWheel(event: WheelEvent): void {
  if (!props.active || (!event.ctrlKey && !event.metaKey)) {
    return;
  }
  event.preventDefault();
  if (event.deltaY === 0) {
    return;
  }
  zoomBy(event.deltaY > 0 ? -1 : 1);
}

function watermarkSize(height: number): number {
  return Math.min(22, Math.max(9, height * 0.42));
}

function bandLabel(kind: string): string {
  return BAND_LABEL[kind as BandKind] ?? kind;
}

function stripHeight(height: number): number {
  return height > 0 ? height : EMPTY_STRIP;
}

function message(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

function pushUndo(): void {
  if (!report.value) {
    return;
  }
  undoStack.value.push(JSON.stringify(report.value));
  if (undoStack.value.length > 50) {
    undoStack.value.shift();
  }
}

function dropNoopUndo(): void {
  const top = undoStack.value[undoStack.value.length - 1];
  if (report.value && top && JSON.stringify(report.value) === top) {
    undoStack.value.pop();
  }
}

function edit(mutate: () => void): void {
  pushUndo();
  mutate();
  dropNoopUndo();
}

function undo(): void {
  const previous = undoStack.value.pop();
  if (!previous) {
    return;
  }
  report.value = JSON.parse(previous) as Report;
  editing.value = null;
}

function isSelected(next: Selection): boolean {
  return JSON.stringify(selection.value) === JSON.stringify(next);
}

function select(next: Selection): void {
  selection.value = next;
  tool.value = null;
  if (next.kind === "band") {
    void nextTick(() => {
      stageEl.value
        ?.querySelector(`[data-band-index="${next.index}"]`)
        ?.scrollIntoView({ block: "nearest", inline: "nearest" });
    });
  }
}

function setBandHeight(raw: string): void {
  if (!report.value || selection.value.kind !== "band") {
    return;
  }
  const band = report.value.bands[selection.value.index];
  if (!band) {
    return;
  }
  const value = Number(raw);
  if (!Number.isFinite(value)) {
    return;
  }
  edit(() => {
    band.height = Math.max(contentBottom(band), value, 0);
  });
}

async function load(): Promise<void> {
  if (!props.templatePath) {
    report.value = null;
    savedSnapshot.value = "";
    return;
  }
  const ticket = ++generation;
  loading.value = true;
  error.value = "";
  try {
    const loaded = await invoke<Report>("load_report", { path: props.templatePath });
    if (ticket !== generation) {
      return;
    }
    syncColumn(loaded);
    report.value = loaded;
    savedSnapshot.value = JSON.stringify(loaded);
    loadWarnings.value = loaded.warnings.map((warning) => ({ ...warning }));
    lossyConfirmed.value =
      loaded.warnings.length === 0 || confirmedPaths.has(props.templatePath);
    undoStack.value = [];
    selection.value = { kind: "report" };
    tool.value = null;
    editing.value = null;
  } catch (err) {
    if (ticket !== generation) {
      return;
    }
    report.value = null;
    error.value = message(err);
  } finally {
    if (ticket === generation) {
      loading.value = false;
    }
  }
}

async function saveDesign(asNew: boolean): Promise<void> {
  if (!report.value || saving.value) {
    return;
  }
  let path = asNew ? "" : props.templatePath;
  if (!path) {
    const picked = await save({
      filters: [{ name: "JRXML", extensions: ["jrxml"] }],
      defaultPath: `${report.value.name || "Report"}.jrxml`,
    });
    if (typeof picked !== "string") {
      return;
    }
    path = picked.toLowerCase().endsWith(".jrxml") ? picked : `${picked}.jrxml`;
  }
  if (loadWarnings.value.length > 0 && !lossyConfirmed.value) {
    const lines = loadWarnings.value
      .map((warning) => `${warning.tag} (${warning.location})`)
      .join("\n");
    if (!window.confirm(`Saving omits unsupported tags:\n\n${lines}\n\nContinue?`)) {
      return;
    }
    confirmedPaths.add(path);
    lossyConfirmed.value = true;
  }
  syncColumn(report.value);
  saving.value = true;
  error.value = "";
  try {
    await invoke("save_report", { path, report: report.value });
    savedSnapshot.value = JSON.stringify(report.value);
    emit("saved", path);
  } catch (err) {
    error.value = message(err);
  } finally {
    saving.value = false;
  }
}

function onToolDragStart(event: DragEvent, id: Tool): void {
  event.dataTransfer?.setData(DRAG_TOOL, id);
  event.dataTransfer?.setData("text/plain", id);
  if (event.dataTransfer) {
    event.dataTransfer.effectAllowed = "copy";
  }
}

function toolFromDrop(transfer: DataTransfer | null): Tool | null {
  if (!transfer) {
    return null;
  }
  const id = transfer.getData(DRAG_TOOL) || transfer.getData("text/plain");
  return palette.some((item) => item.id === id) ? (id as Tool) : null;
}

function addBand(): void {
  if (!report.value || !bandToAdd.value) {
    return;
  }
  const kind = bandToAdd.value;
  edit(() => {
    const index = ensureBand(report.value!, kind);
    selection.value = { kind: "band", index };
  });
  bandToAdd.value = "";
}

function onFieldDragStart(event: DragEvent, name: string): void {
  event.dataTransfer?.setData(DRAG_FIELD, name);
  event.dataTransfer?.setData("text/plain", name);
  if (event.dataTransfer) {
    event.dataTransfer.effectAllowed = "copy";
  }
}

function bandAtPoint(clientX: number, clientY: number): { index: number; x: number; y: number } | null {
  const root = sheetEl.value;
  if (!root) {
    return null;
  }
  for (const row of root.querySelectorAll<HTMLElement>("[data-band-index]")) {
    const content = row.querySelector<HTMLElement>("[data-band-content]") ?? row;
    const bounds = content.getBoundingClientRect();
    if (clientX < bounds.left || clientX > bounds.right || clientY < bounds.top || clientY > bounds.bottom) {
      continue;
    }
    const index = Number(row.dataset.bandIndex);
    if (!Number.isInteger(index)) {
      return null;
    }
    return {
      index,
      x: (clientX - bounds.left) / scale.value,
      y: (clientY - bounds.top) / scale.value,
    };
  }
  return null;
}

function onCanvasDragOver(event: DragEvent): void {
  if (!event.dataTransfer) {
    return;
  }
  event.preventDefault();
  event.dataTransfer.dropEffect = "copy";
  dropBand.value = bandAtPoint(event.clientX, event.clientY)?.index ?? null;
}

function onCanvasDragLeave(event: DragEvent): void {
  const next = event.relatedTarget as Node | null;
  if (next && (event.currentTarget as HTMLElement).contains(next)) {
    return;
  }
  dropBand.value = null;
}

function onCanvasDrop(event: DragEvent): void {
  event.preventDefault();
  const hit = bandAtPoint(event.clientX, event.clientY);
  dropBand.value = null;
  if (!hit || !report.value) {
    return;
  }
  const fieldName = event.dataTransfer?.getData(DRAG_FIELD) ?? "";
  const armed = toolFromDrop(event.dataTransfer);
  if (!armed && !fieldName) {
    return;
  }
  edit(() => {
    const index = armed
      ? placeElement(report.value!, hit.index, armed, hit.x, hit.y)
      : fieldName
        ? placeField(report.value!, hit.index, fieldName, hit.x, hit.y)
        : null;
    if (index === null) {
      if (armed === "table") {
        error.value = "Tables go on the Detail band.";
      }
      return;
    }
    error.value = "";
    selection.value = { kind: "element", band: hit.index, index };
    tool.value = null;
  });
}

function onBandClick(event: MouseEvent, bandIndex: number): void {
  if (!report.value) {
    return;
  }
  if (!tool.value) {
    select({ kind: "band", index: bandIndex });
    return;
  }
  const bounds = (event.currentTarget as HTMLElement).getBoundingClientRect();
  const armed = tool.value;
  edit(() => {
    const index = placeElement(
      report.value!,
      bandIndex,
      armed,
      (event.clientX - bounds.left) / scale.value,
      (event.clientY - bounds.top) / scale.value,
    );
    if (index === null) {
      if (armed === "table") {
        error.value = "Tables go on the Detail band.";
      }
      return;
    }
    error.value = "";
    selection.value = { kind: "element", band: bandIndex, index };
    tool.value = null;
  });
}

function onElementPointerDown(event: PointerEvent, bandIndex: number, index: number): void {
  if (!report.value || editing.value) {
    return;
  }
  selection.value = { kind: "element", band: bandIndex, index };
  tool.value = null;
  const element = report.value.bands[bandIndex].elements[index];
  pushUndo();
  drag = {
    kind: "move",
    band: bandIndex,
    index,
    originX: element.element.x,
    originY: element.element.y,
    pointerX: event.clientX,
    pointerY: event.clientY,
  };
  (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
}

function onHandlePointerDown(event: PointerEvent, bandIndex: number, index: number, handle: Handle): void {
  if (!report.value) {
    return;
  }
  const element = report.value.bands[bandIndex].elements[index];
  pushUndo();
  drag = {
    kind: "resize",
    band: bandIndex,
    index,
    handle,
    originX: element.element.x,
    originY: element.element.y,
    originW: element.element.width,
    originH: element.element.height,
    pointerX: event.clientX,
    pointerY: event.clientY,
  };
  (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
}

function onBandPointerDown(event: PointerEvent, bandIndex: number): void {
  if (!report.value) {
    return;
  }
  selection.value = { kind: "band", index: bandIndex };
  tool.value = null;
  pushUndo();
  drag = {
    kind: "band",
    band: bandIndex,
    origin: report.value.bands[bandIndex].height,
    pointerY: event.clientY,
  };
  (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
}

function clampPanel(width: number): number {
  const limit = Math.min(480, Math.max(220, window.innerWidth * 0.45));
  return Math.min(limit, Math.max(160, Math.round(width)));
}

function onPanelResizeDown(event: PointerEvent, side: "outline" | "inspector"): void {
  panelDrag = {
    side,
    startX: event.clientX,
    startWidth: side === "outline" ? outlineWidth.value : inspectorWidth.value,
  };
  (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
}

function onPointerMove(event: PointerEvent): void {
  trackPointer(event);
  if (panelDrag) {
    const delta = event.clientX - panelDrag.startX;
    const width = clampPanel(
      panelDrag.side === "outline" ? panelDrag.startWidth + delta : panelDrag.startWidth - delta,
    );
    if (panelDrag.side === "outline") {
      outlineWidth.value = width;
    } else {
      inspectorWidth.value = width;
    }
    return;
  }
  if (!drag || !report.value) {
    return;
  }
  const band = report.value.bands[drag.band];
  if (!band) {
    return;
  }
  if (drag.kind === "band") {
    const height = Math.round(drag.origin + (event.clientY - drag.pointerY) / scale.value);
    band.height = Math.max(contentBottom(band), height, 0);
    return;
  }
  const element = band.elements[drag.index];
  if (!element) {
    return;
  }
  const screenX = event.clientX - drag.pointerX;
  const screenY = event.clientY - drag.pointerY;
  if (Math.abs(screenX) < 3 && Math.abs(screenY) < 3) {
    return;
  }
  const dx = screenX / scale.value;
  const dy = screenY / scale.value;
  if (drag.kind === "move") {
    element.element.x = snap(drag.originX + dx);
    element.element.y = snap(drag.originY + dy);
    clampElement(element.element, report.value.columnWidth, band.height);
    return;
  }
  element.element.x = drag.originX;
  element.element.y = drag.originY;
  element.element.width = drag.originW;
  element.element.height = drag.originH;
  applyResize(element.element, drag.handle, dx, dy, report.value.columnWidth, band.height);
}

function onPointerUp(): void {
  panelDrag = null;
  if (!drag) {
    return;
  }
  drag = null;
  dropNoopUndo();
}

function onDoubleClick(bandIndex: number, index: number): void {
  const element = report.value?.bands[bandIndex].elements[index];
  if (!element || element.type !== "staticText") {
    return;
  }
  textDraft.value = element.text;
  editing.value = { band: bandIndex, index };
}

function commitText(): void {
  if (!editing.value || !report.value) {
    return;
  }
  const { band, index } = editing.value;
  const element = report.value.bands[band]?.elements[index];
  editing.value = null;
  if (!element || element.type !== "staticText" || element.text === textDraft.value) {
    return;
  }
  edit(() => {
    if (element.type === "staticText") {
      element.text = textDraft.value;
    }
  });
}

function sectionLabel(kind: SectionKind): string {
  return SECTION_LABEL[kind];
}

function chartKindLabel(kind: string): string {
  return CHART_KINDS.find((item) => item.id === kind)?.label ?? kind;
}

async function loadJson(path: string): Promise<void> {
  jsonDocument.value = null;
  jsonError.value = "";
  if (!path) {
    return;
  }
  try {
    const text = await invoke<string>("read_template", { path });
    jsonDocument.value = JSON.parse(text) as unknown;
  } catch (err) {
    jsonError.value = message(err);
  }
}

function directoryOf(file: string): string {
  const index = Math.max(file.lastIndexOf("/"), file.lastIndexOf("\\"));
  return index >= 0 ? file.slice(0, index) : "";
}

function joinPath(dir: string, rel: string): string {
  if (!rel || rel.startsWith("/") || /^[A-Za-z]:[\\/]/.test(rel)) {
    return rel;
  }
  const slash = dir.includes("\\") && !dir.includes("/") ? "\\" : "/";
  const parts = `${dir}${slash}${rel}`.split(/[/\\]/);
  const stack: string[] = [];
  for (const part of parts) {
    if (!part || part === ".") {
      continue;
    }
    if (part === "..") {
      stack.pop();
    } else {
      stack.push(part);
    }
  }
  return `${dir.startsWith("/") ? "/" : ""}${stack.join(slash)}`;
}

function relativePath(fromDir: string, target: string): string {
  const slash = fromDir.includes("\\") && !fromDir.includes("/") ? "\\" : "/";
  const from = fromDir.split(/[/\\]/).filter(Boolean);
  const to = target.split(/[/\\]/).filter(Boolean);
  let shared = 0;
  while (shared < from.length && shared < to.length && from[shared] === to[shared]) {
    shared += 1;
  }
  return [...from.slice(shared).map(() => ".."), ...to.slice(shared)].join(slash) || target;
}

function subreportPathText(expression: string): string {
  const trimmed = expression.trim();
  if (trimmed.startsWith('"') && trimmed.endsWith('"') && trimmed.length >= 2) {
    return trimmed.slice(1, -1).replace(/\\"/g, '"');
  }
  return trimmed;
}

function setSubreportSource(path: string): void {
  edit(() => {
    if (editedElement.value?.type === "subreport") {
      editedElement.value.dataSource = path.trim() || null;
    }
  });
}

async function browseSubreport(): Promise<void> {
  const picked = await open({ multiple: false, filters: [{ name: "JRXML", extensions: ["jrxml"] }] });
  if (typeof picked !== "string") {
    return;
  }
  const relative = props.templatePath ? relativePath(directoryOf(props.templatePath), picked) : picked;
  edit(() => {
    if (editedElement.value?.type === "subreport") {
      editedElement.value.expression = chartExpression(relative);
    }
  });
}

async function createSubreport(): Promise<void> {
  const suggested = props.templatePath ? joinPath(directoryOf(props.templatePath), "subreport.jrxml") : "subreport.jrxml";
  const picked = await save({ defaultPath: suggested, filters: [{ name: "JRXML", extensions: ["jrxml"] }] });
  if (typeof picked !== "string") {
    return;
  }
  try {
    await invoke("save_report", { path: picked, report: blankReport() });
  } catch (err) {
    error.value = message(err);
    return;
  }
  const relative = props.templatePath ? relativePath(directoryOf(props.templatePath), picked) : picked;
  edit(() => {
    if (editedElement.value?.type === "subreport") {
      editedElement.value.expression = chartExpression(relative);
    }
  });
}

function openSubreport(expression: string): void {
  const text = subreportPathText(expression);
  if (!text || !props.templatePath) {
    return;
  }
  const file = text.endsWith(".jasper") ? `${text.slice(0, -".jasper".length)}.jrxml` : text;
  emit("openTemplate", joinPath(directoryOf(props.templatePath), file));
}

function setChartExpression(
  chart: Extract<Element, { type: "chart" }>,
  series: CategorySeries,
  key: "seriesExpression" | "categoryExpression" | "valueExpression",
  value: string,
): void {
  if (!report.value) {
    return;
  }
  series[key] = chartExpression(value);
  rememberChartFields(report.value, chart);
}

function addChartSeries(chart: Extract<Element, { type: "chart" }>): void {
  const series = chart.series ?? [];
  chart.series = series;
  const index = series.length + 1;
  series.push({
    seriesExpression: `"SERIES ${index}"`,
    categoryExpression: series[0]?.categoryExpression ?? "0",
    valueExpression: "0",
  });
}

function clampEdited(element: ReportElement): void {
  if (!report.value) {
    return;
  }
  const current = selection.value;
  if (current.kind === "element") {
    const band = report.value.bands[current.band];
    if (band) {
      clampElement(element, report.value.columnWidth, band.height);
    }
    return;
  }
  if (current.kind === "cell" && selectedTable.value) {
    const column = selectedTable.value.columns[current.column];
    const section = column?.sections[current.section];
    if (column && section) {
      clampElement(element, column.width, section.height);
    }
  }
}

function removeSelected(): void {
  if (!report.value) {
    return;
  }
  const current = selection.value;
  if (current.kind === "band") {
    edit(() => {
      report.value!.bands.splice(current.index, 1);
    });
    selection.value = { kind: "report" };
    return;
  }
  if (current.kind === "element") {
    edit(() => {
      report.value!.bands[current.band].elements.splice(current.index, 1);
    });
    selection.value = { kind: "band", index: current.band };
    return;
  }
  if (current.kind === "column" && selectedTable.value) {
    const table = selectedTable.value;
    edit(() => removeColumn(table, current.column));
    selection.value = { kind: "element", band: current.band, index: current.index };
    return;
  }
  if (current.kind === "cell" && selectedTable.value) {
    const section = selectedTable.value.columns[current.column]?.sections[current.section];
    if (!section) {
      return;
    }
    edit(() => {
      section.elements.splice(current.element, 1);
    });
    selection.value = {
      kind: "columnSection",
      band: current.band,
      index: current.index,
      column: current.column,
      section: current.section,
    };
  }
}

function nudgeSelected(dx: number, dy: number): void {
  if (!report.value || !editedElement.value) {
    return;
  }
  const element = editedElement.value;
  if (selection.value.kind === "element") {
    const band = report.value.bands[selection.value.band];
    edit(() => {
      nudge(element.element, dx, dy, report.value!.columnWidth, band.height);
    });
    return;
  }
  if (selection.value.kind === "cell" && selectedTable.value) {
    const column = selectedTable.value.columns[selection.value.column];
    const section = column?.sections[selection.value.section];
    if (!column || !section) {
      return;
    }
    edit(() => {
      nudge(element.element, dx, dy, column.width, section.height);
    });
  }
}

function onKeydown(event: KeyboardEvent): void {
  if (!props.active) {
    return;
  }
  const target = event.target as HTMLElement | null;
  const typing = !!target?.closest("input, textarea, select");
  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "s") {
    event.preventDefault();
    void saveDesign(false);
    return;
  }
  if ((event.metaKey || event.ctrlKey) && (event.key === "=" || event.key === "+")) {
    event.preventDefault();
    zoomBy(1);
    return;
  }
  if ((event.metaKey || event.ctrlKey) && event.key === "-") {
    event.preventDefault();
    zoomBy(-1);
    return;
  }
  if ((event.metaKey || event.ctrlKey) && event.key === "0") {
    event.preventDefault();
    scale.value = 1;
    return;
  }
  if (typing) {
    return;
  }
  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "z" && !event.shiftKey) {
    event.preventDefault();
    undo();
    return;
  }
  if (event.key === "Delete" || event.key === "Backspace") {
    event.preventDefault();
    removeSelected();
    return;
  }
  const step = event.shiftKey ? 10 : 1;
  if (event.key === "ArrowLeft") {
    event.preventDefault();
    nudgeSelected(-step, 0);
  } else if (event.key === "ArrowRight") {
    event.preventDefault();
    nudgeSelected(step, 0);
  } else if (event.key === "ArrowUp") {
    event.preventDefault();
    nudgeSelected(0, -step);
  } else if (event.key === "ArrowDown") {
    event.preventDefault();
    nudgeSelected(0, step);
  }
}

function commitNumber(raw: string, apply: (value: number) => void): void {
  const value = Number(raw);
  if (!Number.isFinite(value)) {
    return;
  }
  edit(() => apply(value));
}

function applyPreset(preset: string): void {
  if (!report.value || preset === "custom") {
    return;
  }
  edit(() => {
    if (!report.value) {
      return;
    }
    if (preset === "a4") {
      report.value.pageWidth = 595;
      report.value.pageHeight = 842;
    } else if (preset === "letter") {
      report.value.pageWidth = 612;
      report.value.pageHeight = 792;
    }
    syncColumn(report.value);
  });
}

function boxStyle(element: Element): Record<string, string> {
  const height =
    element.type === "line" || element.type === "break"
      ? Math.max(element.element.height, 8)
      : element.element.height;
  return {
    left: `${element.element.x}px`,
    top: `${element.element.y}px`,
    width: `${element.element.width}px`,
    height: `${height}px`,
    zIndex: "1",
  };
}

function guideStyle(element: Element): Record<string, string> {
  if (element.type === "line" || element.type === "ellipse" || borderWidth(element.element) > 0) {
    return {};
  }
  return {
    outline: "1px solid rgba(148, 163, 184, 0.55)",
    outlineOffset: "-1px",
  };
}

function borderStyle(element: Element): Record<string, string> {
  if (element.type === "line" || element.type === "ellipse") {
    return {};
  }
  const width = borderWidth(element.element);
  if (width <= 0) {
    return {};
  }
  return {
    boxSizing: "border-box",
    borderWidth: `${width}px`,
    borderStyle: "solid",
    borderColor: hexColor(borderColor(element.element)),
  };
}

function quotedImageSource(expression: string): string {
  const trimmed = expression.trim();
  if (trimmed.length < 2 || !trimmed.startsWith('"') || !trimmed.endsWith('"')) {
    return "";
  }
  return trimmed.slice(1, -1);
}

function imageObjectStyle(element: Element): Record<string, string> {
  if (element.type !== "image") {
    return {};
  }
  const scale = element.scaleImage ?? "retainShape";
  const fit = scale === "fillFrame" ? "fill" : scale === "clip" ? "none" : "contain";
  const horizontal = element.hAlign === "center" ? "center" : element.hAlign === "right" ? "right" : "left";
  const vertical = element.vAlign === "middle" ? "center" : element.vAlign === "bottom" ? "bottom" : "top";
  const deg = element.rotation === "left" ? -90 : element.rotation === "right" ? 90 : element.rotation === "upsideDown" ? 180 : 0;
  return {
    objectFit: fit,
    objectPosition: `${horizontal} ${vertical}`,
    transform: deg ? `rotate(${deg}deg)` : "none",
  };
}

function quotedImageSources(elements: Element[]): string[] {
  const found: string[] = [];
  for (const element of elements) {
    if (element.type === "image") {
      const source = quotedImageSource(element.expression);
      if (source) {
        found.push(source);
      }
    }
    if (element.type === "table") {
      for (const column of element.columns) {
        for (const section of column.sections) {
          found.push(...quotedImageSources(section.elements));
        }
      }
    }
  }
  return found;
}

function imagePreview(element: Element): string {
  if (element.type !== "image") {
    return "";
  }
  const source = quotedImageSource(element.expression);
  return source ? (imagePreviews.value[source] ?? "") : "";
}

async function loadImagePreviews(): Promise<void> {
  const ticket = ++imagePreviewTicket;
  const sources = new Set(quotedImageSources((report.value?.bands ?? []).flatMap((band) => band.elements)));
  const next: Record<string, string> = {};
  await Promise.all(
    [...sources].map(async (source) => {
      if (/^https?:\/\//i.test(source) || source.startsWith("data:")) {
        next[source] = source;
        return;
      }
      if (!props.templatePath) {
        next[source] = "";
        return;
      }
      try {
        next[source] = await invoke<string>("read_image", {
          templatePath: props.templatePath,
          source,
        });
      } catch {
        next[source] = "";
      }
    }),
  );
  if (ticket === imagePreviewTicket) {
    imagePreviews.value = next;
  }
}

function commitImageExpression(raw: string): string {
  const trimmed = raw.trim();
  if (!trimmed || trimmed.startsWith('"') || trimmed.startsWith("$") || trimmed.includes("(")) {
    return trimmed;
  }
  return `"${trimmed.replaceAll('"', "")}"`;
}

function fillStyle(element: Element): Record<string, string> {
  if (element.type === "ellipse" || element.element.mode !== "opaque") {
    return {};
  }
  const color = element.element.backcolor ?? { r: 255, g: 255, b: 255 };
  return { backgroundColor: hexColor(color) };
}

function textFrameStyle(): Record<string, string> {
  return { position: "relative", height: "100%", overflow: "hidden" };
}

function textRotateStyle(element: Element): Record<string, string> {
  if (element.type !== "staticText" && element.type !== "textField") {
    return {};
  }
  const borders = element.element.borders;
  const pad = {
    top: borders?.paddingTop ?? 0,
    left: borders?.paddingLeft ?? 0,
    bottom: borders?.paddingBottom ?? 0,
    right: borders?.paddingRight ?? 0,
  };
  const innerW = Math.max(0, element.element.width - pad.left - pad.right);
  const innerH = Math.max(0, element.element.height - pad.top - pad.bottom);
  const rotation = element.textStyle.rotation;
  const swap = rotation === "left" || rotation === "right";
  const deg = rotation === "left" ? -90 : rotation === "right" ? 90 : rotation === "upsideDown" ? 180 : 0;
  const vertical = element.textStyle.verticalAlignment;
  return {
    position: "absolute",
    left: `${pad.left + (rotation ? innerW / 2 : 0)}px`,
    top: `${pad.top + (rotation ? innerH / 2 : 0)}px`,
    width: `${swap ? innerH : innerW}px`,
    height: `${swap ? innerW : innerH}px`,
    transform: rotation ? `translate(-50%, -50%) rotate(${deg}deg)` : "none",
    display: "flex",
    flexDirection: "column",
    justifyContent: vertical === "middle" ? "center" : vertical === "bottom" ? "flex-end" : "flex-start",
    overflow: "hidden",
    boxSizing: "border-box",
    paddingLeft: `${element.textStyle.leftIndent ?? 0}px`,
    paddingRight: `${element.textStyle.rightIndent ?? 0}px`,
  };
}

function textContentStyle(element: Element): Record<string, string> {
  if (element.type !== "staticText" && element.type !== "textField") {
    return {};
  }
  const align = element.textStyle.textAlignment ?? "left";
  return {
    display: "block",
    width: "100%",
    fontSize: `${element.textStyle.fontSize ?? 10}px`,
    fontWeight: element.textStyle.bold ? "700" : "400",
    fontStyle: element.textStyle.italic ? "italic" : "normal",
    textDecoration: decoration(!!element.textStyle.underline, !!element.textStyle.strikeThrough),
    fontFamily: element.textStyle.fontName
      ? `"${element.textStyle.fontName}", "Liberation Sans", sans-serif`
      : '"Liberation Sans", sans-serif',
    lineHeight: lineHeightOf(element),
    textAlign: align === "justified" ? "justify" : align,
    textAlignLast: align === "justified" ? "left" : align,
    textIndent: `${element.textStyle.firstLineIndent ?? 0}px`,
    marginTop: `${element.textStyle.spacingBefore ?? 0}px`,
    tabSize: `${element.textStyle.tabStopWidth ?? 40}px`,
    whiteSpace: "pre-wrap",
    overflowWrap: "anywhere",
  };
}

const FONT_ASCENT = 1854 / 2048;
const FONT_DESCENT = 434 / 2048;
const FONT_LEADING = 67 / 2048;

function decoration(underline: boolean, strike: boolean): string {
  const parts = [underline ? "underline" : "", strike ? "line-through" : ""].filter(Boolean);
  return parts.join(" ") || "none";
}

function lineHeightOf(element: Element): string {
  if (element.type !== "staticText" && element.type !== "textField") {
    return "1";
  }
  const size = element.textStyle.fontSize ?? 10;
  const ascent = FONT_ASCENT * size;
  const descent = FONT_DESCENT * size;
  const leading = FONT_LEADING * size;
  const single = leading + ascent + descent;
  const spacing = element.textStyle.lineSpacingSize ?? size;
  switch (element.textStyle.lineSpacing) {
    case "oneAndHalf":
      return `${descent + leading + 1.5 * ascent}px`;
    case "double":
      return `${descent + leading + 2 * ascent}px`;
    case "proportional":
      return `${descent + leading + (element.textStyle.lineSpacingSize ?? 1) * ascent}px`;
    case "fixed":
      return `${spacing}px`;
    case "atLeast":
      return `${Math.max(single, spacing)}px`;
    default:
      return `${single}px`;
  }
}

function textPieces(element: Element): { text: string; style: Record<string, string> }[] {
  if (element.type !== "staticText" && element.type !== "textField") {
    return [{ text: "", style: {} }];
  }
  const label = elementLabel(element);
  if (element.textStyle.markup !== "styled" && element.textStyle.markup !== "html") {
    return [{ text: label, style: {} }];
  }
  return parseMarkupPieces(label);
}

function parseMarkupPieces(text: string): { text: string; style: Record<string, string> }[] {
  const pieces: { text: string; style: Record<string, string> }[] = [];
  const stack: Record<string, string>[] = [{}];
  let buf = "";
  const flush = () => {
    if (!buf) return;
    pieces.push({ text: buf, style: { ...stack[stack.length - 1] } });
    buf = "";
  };
  const pattern = /<\/?([a-zA-Z0-9]+)([^>]*)>|([^<]+)/g;
  let match: RegExpExecArray | null;
  while ((match = pattern.exec(text))) {
    if (match[3]) {
      buf += match[3].replace(/&amp;/g, "&").replace(/&lt;/g, "<").replace(/&gt;/g, ">").replace(/&quot;/g, '"');
      continue;
    }
    flush();
    const name = match[1].toLowerCase();
    const closing = match[0].startsWith("</");
    const attrs = match[2] ?? "";
    if (closing) {
      if (stack.length > 1) stack.pop();
      continue;
    }
    if (name === "br") {
      buf = "\n";
      flush();
      continue;
    }
    const next = { ...stack[stack.length - 1] };
    const flag = (key: string) => new RegExp(`${key}="(true|false)"`, "i").exec(attrs)?.[1];
    if (name === "b" || name === "strong" || flag("isBold") === "true") next.fontWeight = "700";
    if (flag("isBold") === "false") delete next.fontWeight;
    if (name === "i" || name === "em" || flag("isItalic") === "true") next.fontStyle = "italic";
    if (flag("isItalic") === "false") delete next.fontStyle;
    const strike = name === "s" || name === "strike" || name === "del" || flag("isStrikeThrough") === "true";
    const underline = name === "u" || flag("isUnderline") === "true";
    if (strike || underline) {
      next.textDecoration = decoration(underline || next.textDecoration === "underline", strike || next.textDecoration === "line-through");
    }
    const color = /(?:forecolor|color)="(#[0-9A-Fa-f]{3,8})"/.exec(attrs)?.[1];
    if (color) next.color = color;
    const size = /size="([0-9.]+)"/.exec(attrs)?.[1];
    if (size) {
      const value = Number(size);
      const html = name === "font" && value >= 1 && value <= 7 && Number.isInteger(value);
      const points = html ? [8, 10, 12, 14, 18, 24, 36][value - 1] : value;
      next.fontSize = `${points}px`;
    }
    if (name === "li") {
      buf = "\n";
      flush();
    }
    if (!match[0].endsWith("/>")) stack.push(next);
  }
  flush();
  return pieces.length ? pieces : [{ text, style: {} }];
}

function sectionChrome(section: ColumnSection): Record<string, string> {
  const named = report.value?.styles.find((item) => item.name === section.style);
  const chrome: Record<string, string> = {};
  if (named?.mode === "opaque") {
    chrome.backgroundColor = hexColor(named.backcolor ?? { r: 255, g: 255, b: 255 });
  }
  const borders = named?.borders;
  const pen = borders?.pen ?? borders?.top ?? borders?.left ?? borders?.bottom ?? borders?.right;
  const width = pen?.lineWidth ?? 0;
  if (width > 0) {
    chrome.boxSizing = "border-box";
    chrome.borderWidth = `${width}px`;
    chrome.borderStyle = "solid";
    chrome.borderColor = hexColor(pen?.lineColor ?? null);
  } else {
    chrome.borderRight = "1px solid rgb(226 232 240)";
    chrome.borderBottom = "1px solid rgb(226 232 240)";
  }
  return chrome;
}

function placedChildStyle(element: Element, columnWidth: number, sectionHeight: number): Record<string, string> {
  const box = element.element;
  const width = columnWidth > 0 ? columnWidth : 1;
  const height = sectionHeight > 0 ? sectionHeight : 1;
  return {
    left: `${(box.x / width) * 100}%`,
    top: `${(box.y / height) * 100}%`,
    width: `${(box.width / width) * 100}%`,
    height: `${(box.height / height) * 100}%`,
  };
}

function kindRowWeight(table: Extract<Element, { type: "table" }>, kind: SectionKind): number {
  return Math.max(
    1,
    table.columns.reduce((max, column) => {
      const section = column.sections.find((item) => item.kind === kind);
      return Math.max(max, section?.height ?? 0);
    }, 0),
  );
}

function handlePosition(bandIndex: number, handle: Handle): Record<string, string> {
  if (!report.value || selection.value.kind !== "element" || selection.value.band !== bandIndex) {
    return { display: "none" };
  }
  const element = report.value.bands[bandIndex]?.elements[selection.value.index];
  if (!element) {
    return { display: "none" };
  }
  const box = element.element;
  const west = handle === "w" || handle === "nw" || handle === "sw";
  const east = handle === "e" || handle === "ne" || handle === "se";
  const north = handle === "n" || handle === "nw" || handle === "ne";
  const south = handle === "s" || handle === "sw" || handle === "se";
  const left = west ? box.x : east ? box.x + box.width : box.x + box.width / 2;
  const top = north ? box.y : south ? box.y + box.height : box.y + box.height / 2;
  return { left: `${left}px`, top: `${top}px` };
}

function addFieldFromProperties(): void {
  if (!report.value) {
    return;
  }
  edit(() => {
    if (!report.value) {
      return;
    }
    const index = addMainField(report.value);
    selection.value = { kind: "field", index };
  });
}

function addDatasetFieldFromProperties(): void {
  if (!report.value || selection.value.kind !== "dataset") {
    return;
  }
  const datasetIndex = selection.value.index;
  edit(() => {
    const dataset = report.value?.subdatasets[datasetIndex];
    if (!dataset) {
      return;
    }
    const index = addDatasetField(dataset);
    selection.value = { kind: "datasetField", dataset: datasetIndex, index };
  });
}

function removeSelectedField(): void {
  if (!report.value) {
    return;
  }
  if (selection.value.kind === "field") {
    report.value.fields.splice(selection.value.index, 1);
    selection.value = { kind: "fields" };
    return;
  }
  if (selection.value.kind === "datasetField") {
    report.value.subdatasets[selection.value.dataset]?.fields.splice(selection.value.index, 1);
    selection.value = { kind: "dataset", index: selection.value.dataset };
  }
}

function choicesFor(element: Element): string[] {
  if (!report.value || element.type !== "textField") {
    return [];
  }
  if (selection.value.kind === "cell" && selectedTable.value) {
    return columnChoices(selectedTable.value, element.expression);
  }
  return expressionChoices(report.value.fields, element.expression, true);
}

function tableChildSelected(band: number, index: number): boolean {
  const current = selection.value;
  return (
    (current.kind === "tableSection" ||
      current.kind === "column" ||
      current.kind === "columnSection" ||
      current.kind === "cell") &&
    current.band === band &&
    current.index === index
  );
}

function selectColumnSection(band: number, index: number, columnIndex: number, kind: SectionKind): void {
  const table = report.value?.bands[band]?.elements[index];
  if (!table || table.type !== "table") {
    return;
  }
  const found = sectionByKind(table.columns[columnIndex], kind);
  if (!found) {
    return;
  }
  select({ kind: "columnSection", band, index, column: columnIndex, section: found.index });
}

function columnCellClass(band: number, index: number, columnIndex: number, kind: SectionKind): string {
  const table = report.value?.bands[band]?.elements[index];
  if (!table || table.type !== "table") {
    return "";
  }
  const found = sectionByKind(table.columns[columnIndex], kind);
  if (!found) {
    return "";
  }
  if (isSelected({ kind: "columnSection", band, index, column: columnIndex, section: found.index })) {
    return "ring-2 ring-inset ring-blue-500";
  }
  if (isSelected({ kind: "tableSection", band, index, section: kind })) {
    return "ring-2 ring-inset ring-blue-300";
  }
  if (isSelected({ kind: "column", band, index, column: columnIndex })) {
    return "ring-1 ring-inset ring-blue-300";
  }
  return "";
}

function selectCell(band: number, index: number, columnIndex: number, kind: SectionKind, elementIndex: number): void {
  const table = report.value?.bands[band]?.elements[index];
  if (!table || table.type !== "table") {
    return;
  }
  const found = sectionByKind(table.columns[columnIndex], kind);
  if (!found) {
    return;
  }
  select({
    kind: "cell",
    band,
    index,
    column: columnIndex,
    section: found.index,
    element: elementIndex,
  });
}

function columnChoices(element: Extract<Element, { type: "table" }>, expression: string): string[] {
  if (!report.value) {
    return expression ? [expression] : [];
  }
  const dataset = report.value.subdatasets.find((item) => item.name === element.subDataset);
  return expressionChoices(dataset?.fields ?? [], expression, false);
}

function confirmDiscardIfDirty(messageText = "Discard unsaved Design changes?"): boolean {
  if (!dirty.value) {
    return true;
  }
  return window.confirm(messageText);
}

function discardChanges(): void {
  if (!savedSnapshot.value) {
    return;
  }
  report.value = JSON.parse(savedSnapshot.value) as Report;
  undoStack.value = [];
  editing.value = null;
}

function isDirty(): boolean {
  return dirty.value;
}

function fieldsFor(id: string): Field[] | null {
  if (!report.value) {
    return null;
  }
  if (id === "report") {
    return report.value.fields;
  }
  const index = Number(id);
  return report.value.subdatasets[index]?.fields ?? null;
}

function appendFields(id: string, found: { name: string; className: string; jsonPath: string }[]): void {
  edit(() => {
    const list = fieldsFor(id);
    if (!list) {
      return;
    }
    const next = fieldsFromDiscovery(list, found);
    list.splice(0, list.length, ...next);
    const current = selection.value;
    if (id === "report" && current.kind === "field" && current.index >= list.length) {
      selection.value = { kind: "fields" };
    }
    if (
      id !== "report" &&
      current.kind === "datasetField" &&
      String(current.dataset) === id &&
      current.index >= list.length
    ) {
      selection.value = { kind: "dataset", index: current.dataset };
    }
  });
}

function changeField(
  id: string,
  index: number,
  patch: { name?: string; className?: string; jsonPath?: string | null },
): void {
  edit(() => {
    const field = fieldsFor(id)?.[index];
    if (!field) {
      return;
    }
    if (patch.name !== undefined) {
      field.name = patch.name;
    }
    if (patch.className !== undefined) {
      field.className = patch.className;
    }
    if ("jsonPath" in patch) {
      field.jsonPath = patch.jsonPath || null;
    }
  });
}

function addField(id: string): void {
  if (!report.value) {
    return;
  }
  if (id === "report") {
    addFieldFromProperties();
    return;
  }
  const index = Number(id);
  edit(() => {
    const dataset = report.value?.subdatasets[index];
    if (!dataset) {
      return;
    }
    const fieldIndex = addDatasetField(dataset);
    selection.value = { kind: "datasetField", dataset: index, index: fieldIndex };
  });
}

function removeField(id: string, index: number): void {
  edit(() => {
    const list = fieldsFor(id);
    if (!list || !list[index]) {
      return;
    }
    list.splice(index, 1);
    if (id === "report") {
      selection.value = { kind: "fields" };
      return;
    }
    const dataset = Number(id);
    selection.value = { kind: "dataset", index: dataset };
  });
}

const datasetChoices = computed(() => {
  const current = report.value;
  if (!current) {
    return [];
  }
  return [
    {
      id: "report",
      name: current.name || "Report",
      fields: current.fields.map((field) => ({ ...field })),
    },
    ...current.subdatasets.map((dataset, index) => ({
      id: String(index),
      name: dataset.name || "Dataset",
      fields: dataset.fields.map((field) => ({ ...field })),
    })),
  ];
});

watch(datasetChoices, (choices) => emit("datasets", choices), { immediate: true });

defineExpose({
  confirmDiscardIfDirty,
  discardChanges,
  isDirty,
  save: () => saveDesign(false),
  saveAs: () => saveDesign(true),
  appendFields,
  changeField,
  addField,
  removeField,
});

watch(
  () => props.templatePath,
  () => {
    void load();
  },
  { immediate: true },
);

watch(
  () =>
    (report.value?.bands ?? [])
      .flatMap((band) => band.elements)
      .flatMap((element) =>
        element.type === "image"
          ? [element.expression]
          : element.type === "table"
            ? element.columns.flatMap((column) => column.sections.flatMap((section) => section.elements.filter((child) => child.type === "image").map((child) => child.expression)))
            : [],
      )
      .join("\n") + props.templatePath,
  () => {
    void loadImagePreviews();
  },
);

watch(
  () => props.active,
  (active) => {
    if (active && !dirty.value) {
      void load();
    }
  },
);

watch(
  () => props.documentPath,
  (path) => {
    void loadJson(path ?? "");
  },
  { immediate: true },
);

watch(editing, async (value) => {
  if (!value) {
    return;
  }
  await nextTick();
  const input = document.querySelector<HTMLInputElement>("[data-design-text]");
  input?.focus();
  input?.select();
});

watch(stageEl, (el, previous) => {
  previous?.removeEventListener("wheel", onWheel);
  el?.addEventListener("wheel", onWheel, { passive: false });
});

watch(
  [scale, canvasWidth, sheetHeight, leftMargin, rightMargin, topMargin, bottomMargin, rulerSpan, pointer, pastPage],
  () => {
    void nextTick(drawRulers);
  },
);

watch([outlineOpen, inspectorOpen, outlineWidth, inspectorWidth], () => {
  void nextTick(syncRulers);
});

onMounted(() => {
  window.addEventListener("pointermove", onPointerMove);
  window.addEventListener("pointerup", onPointerUp);
  window.addEventListener("keydown", onKeydown);
});

onBeforeUnmount(() => {
  stageEl.value?.removeEventListener("wheel", onWheel);
  window.removeEventListener("pointermove", onPointerMove);
  window.removeEventListener("pointerup", onPointerUp);
  window.removeEventListener("keydown", onKeydown);
});
</script>

<template>
  <section class="flex h-full min-h-0 flex-col bg-slate-950 text-gray-100">
    <div class="flex shrink-0 items-center gap-2 border-b border-slate-800 bg-slate-900 px-3 py-2">
      <FwbButton size="xs" color="alternative" :disabled="undoStack.length === 0" @click="undo">
        Undo
      </FwbButton>
      <span v-if="dirty" class="text-xs font-medium text-amber-400">Unsaved changes</span>
      <span v-if="error" class="min-w-0 flex-1 truncate text-xs text-red-400">{{ error }}</span>
      <div class="ml-auto flex items-center gap-1">
        <FwbButtonGroup>
          <FwbButton size="xs" color="alternative" title="Zoom out" :disabled="!report" @click="zoomBy(-1)">
            <Icon name="zoom-out" />
          </FwbButton>
          <select
            class="h-7 min-w-16 border border-gray-600 bg-gray-700 px-2 text-xs text-white outline-none disabled:opacity-50"
            :value="String(scale)"
            :disabled="!report"
            @change="setZoom(($event.target as HTMLSelectElement).value)"
          >
            <option v-if="!ZOOM_STEPS.includes(scale)" :value="String(scale)">
              {{ Math.round(scale * 100) }}%
            </option>
            <option v-for="option in zoomOptions" :key="option.value" :value="option.value">
              {{ option.name }}
            </option>
          </select>
          <FwbButton size="xs" color="alternative" title="Zoom in" :disabled="!report" @click="zoomBy(1)">
            <Icon name="zoom-in" />
          </FwbButton>
        </FwbButtonGroup>
      </div>
    </div>

    <div v-if="loading && !report" class="flex flex-1 items-center justify-center">
      <FwbP class="text-sm text-gray-400">Loading the template…</FwbP>
    </div>
    <div v-else-if="!report" class="flex flex-1 items-center justify-center px-6">
      <FwbP class="text-sm text-gray-400">{{ error || "No template is open." }}</FwbP>
    </div>

    <div v-else class="flex min-h-0 flex-1">
      <button
        v-if="!outlineOpen"
        type="button"
        class="flex w-8 shrink-0 flex-col items-center gap-2 border-r border-slate-800 bg-slate-950 py-2 text-gray-400 hover:bg-slate-900 hover:text-white"
        title="Show outline"
        @click="outlineOpen = true"
      >
        <Icon name="chevron-right" />
        <span class="text-[10px] font-semibold uppercase tracking-wide [writing-mode:vertical-rl]">Outline</span>
      </button>
      <aside
        v-else
        class="relative flex shrink-0 flex-col border-r border-slate-800"
        :style="{ width: `${outlineWidth}px` }"
      >
        <div class="flex items-center justify-between border-b border-slate-800 px-3 py-2 text-xs font-semibold uppercase tracking-wide text-gray-400">
          Outline
          <button
            type="button"
            class="rounded p-0.5 text-gray-400 hover:bg-slate-800 hover:text-white"
            title="Hide outline"
            @click="outlineOpen = false"
          >
            <Icon name="chevron-left" />
          </button>
        </div>
        <div
          class="absolute top-0 -right-1 z-20 h-full w-2 cursor-col-resize hover:bg-blue-500"
          role="separator"
          aria-orientation="vertical"
          title="Resize outline"
          @pointerdown.stop="onPanelResizeDown($event, 'outline')"
        />
        <nav class="min-h-0 flex-1 overflow-auto px-2 py-2 text-sm">
          <button
            type="button"
            class="flex w-full rounded-md px-2 py-1.5 text-left font-medium"
            :class="isSelected({ kind: 'report' }) ? 'bg-blue-600 text-white' : 'hover:bg-slate-800'"
            @click="select({ kind: 'report' })"
          >
            {{ report.name || "Report" }}
          </button>
          <button
            type="button"
            class="mt-1 flex w-full rounded-md px-2 py-1 text-left"
            :class="isSelected({ kind: 'fields' }) ? 'bg-blue-600 text-white' : 'text-gray-300 hover:bg-slate-800'"
            @click="select({ kind: 'fields' })"
          >
            Fields
          </button>
          <button
            v-for="(field, index) in report.fields"
            :key="`field-${index}`"
            type="button"
            class="ml-3 flex w-[calc(100%-0.75rem)] cursor-grab rounded-md px-2 py-1 text-left text-gray-400 active:cursor-grabbing"
            :class="isSelected({ kind: 'field', index }) ? 'bg-blue-600 text-white' : 'hover:bg-slate-800 hover:text-gray-200'"
            draggable="true"
            @dragstart="onFieldDragStart($event, field.name)"
            @click="select({ kind: 'field', index })"
          >
            <span class="truncate">{{ field.name || "Field" }}</span>
          </button>
          <template v-for="(dataset, datasetIndex) in report.subdatasets" :key="`ds-${datasetIndex}`">
            <button
              type="button"
              class="mt-1 flex w-full rounded-md px-2 py-1 text-left"
              :class="isSelected({ kind: 'dataset', index: datasetIndex }) ? 'bg-blue-600 text-white' : 'text-gray-300 hover:bg-slate-800'"
              @click="select({ kind: 'dataset', index: datasetIndex })"
            >
              <span class="truncate">{{ dataset.name }}</span>
            </button>
            <button
              v-for="(field, index) in dataset.fields"
              :key="`ds-${datasetIndex}-${index}`"
              type="button"
              class="ml-3 flex w-[calc(100%-0.75rem)] cursor-grab rounded-md px-2 py-1 text-left text-gray-400 active:cursor-grabbing"
              :class="
                isSelected({ kind: 'datasetField', dataset: datasetIndex, index })
                  ? 'bg-blue-600 text-white'
                  : 'hover:bg-slate-800 hover:text-gray-200'
              "
              draggable="true"
              @dragstart="onFieldDragStart($event, field.name)"
              @click="select({ kind: 'datasetField', dataset: datasetIndex, index })"
            >
              <span class="truncate">{{ field.name }}</span>
            </button>
          </template>
          <template v-for="row in bandRows" :key="row.band.kind">
            <button
              type="button"
              class="mt-1 flex w-full rounded-md px-2 py-1 text-left"
              :class="isSelected({ kind: 'band', index: row.index }) ? 'bg-blue-600 text-white' : 'text-gray-300 hover:bg-slate-800'"
              @click="select({ kind: 'band', index: row.index })"
            >
              {{ bandLabel(row.band.kind) }}
            </button>
            <template v-for="(element, index) in row.band.elements" :key="`${row.index}-${index}`">
              <button
                type="button"
                class="ml-3 flex w-[calc(100%-0.75rem)] rounded-md px-2 py-1 text-left text-gray-400"
                :class="
                  isSelected({ kind: 'element', band: row.index, index })
                    ? 'bg-blue-600 text-white'
                    : 'hover:bg-slate-800 hover:text-gray-200'
                "
                @click="select({ kind: 'element', band: row.index, index })"
              >
                <span class="truncate">{{ elementLabel(element) }}</span>
              </button>
              <template v-if="element.type === 'table'">
                <button
                  v-for="kind in tableSectionKinds(element)"
                  :key="`${row.index}-${index}-section-${kind}`"
                  type="button"
                  class="ml-6 flex w-[calc(100%-1.5rem)] rounded-md px-2 py-1 text-left text-gray-400"
                  :class="
                    isSelected({ kind: 'tableSection', band: row.index, index, section: kind })
                      ? 'bg-blue-600 text-white'
                      : 'hover:bg-slate-800 hover:text-gray-200'
                  "
                  @click="select({ kind: 'tableSection', band: row.index, index, section: kind })"
                >
                  <span class="truncate">{{ sectionLabel(kind) }}</span>
                </button>
                <div class="ml-6 px-2 py-1 text-xs text-gray-500">Columns</div>
                <template v-for="(column, columnIndex) in element.columns" :key="`${row.index}-${index}-col-${columnIndex}`">
                  <button
                    type="button"
                    class="ml-9 flex w-[calc(100%-2.25rem)] rounded-md px-2 py-1 text-left text-gray-400"
                    :class="
                      isSelected({ kind: 'column', band: row.index, index, column: columnIndex })
                        ? 'bg-blue-600 text-white'
                        : 'hover:bg-slate-800 hover:text-gray-200'
                    "
                    @click="select({ kind: 'column', band: row.index, index, column: columnIndex })"
                  >
                    <span class="truncate">Column {{ columnIndex + 1 }}</span>
                  </button>
                  <template v-for="found in orderedSections(column)" :key="`${row.index}-${index}-col-${columnIndex}-${found.index}`">
                    <button
                      type="button"
                      class="ml-12 flex w-[calc(100%-3rem)] rounded-md px-2 py-1 text-left text-gray-500"
                      :class="
                        isSelected({
                          kind: 'columnSection',
                          band: row.index,
                          index,
                          column: columnIndex,
                          section: found.index,
                        })
                          ? 'bg-blue-600 text-white'
                          : 'hover:bg-slate-800 hover:text-gray-200'
                      "
                      @click="select({ kind: 'columnSection', band: row.index, index, column: columnIndex, section: found.index })"
                    >
                      <span class="truncate">{{ sectionLabel(found.section.kind) }}</span>
                    </button>
                    <button
                      v-for="(child, elementIndex) in found.section.elements"
                      :key="`${row.index}-${index}-col-${columnIndex}-${found.index}-${elementIndex}`"
                      type="button"
                      class="ml-[3.75rem] flex w-[calc(100%-3.75rem)] rounded-md px-2 py-1 text-left text-gray-400"
                      :class="
                        isSelected({
                          kind: 'cell',
                          band: row.index,
                          index,
                          column: columnIndex,
                          section: found.index,
                          element: elementIndex,
                        })
                          ? 'bg-blue-600 text-white'
                          : 'hover:bg-slate-800 hover:text-gray-200'
                      "
                      @click="select({ kind: 'cell', band: row.index, index, column: columnIndex, section: found.index, element: elementIndex })"
                    >
                      <span class="truncate">{{ elementLabel(child) }}</span>
                    </button>
                  </template>
                </template>
              </template>
            </template>
          </template>
          <label v-if="missingBands.length" class="mt-3 block px-2 text-xs text-gray-500">
            Add band
            <select v-model="bandToAdd" :class="inputClass" class="mt-1">
              <option value="">Choose</option>
              <option v-for="kind in missingBands" :key="kind" :value="kind">{{ bandLabel(kind) }}</option>
            </select>
          </label>
          <button
            v-if="bandToAdd"
            type="button"
            class="mt-1 flex w-full rounded-md px-2 py-1 text-left text-gray-300 hover:bg-slate-800"
            @click="addBand"
          >
            Add {{ bandLabel(bandToAdd) }}
          </button>
        </nav>
      </aside>

      <div class="flex min-h-0 min-w-0 flex-1 flex-col">
        <div class="flex shrink-0">
          <div
            class="shrink-0 border-r border-b border-slate-700 bg-slate-800"
            :style="{ width: `${RULER}px`, height: `${RULER}px` }"
          />
          <div
            ref="hRulerView"
            class="min-w-0 flex-1 overflow-hidden border-b border-slate-700 bg-slate-900"
            @wheel="onRulerWheel"
          >
            <div :style="{ width: `${STAGE_PAD + canvasWidth * scale + STAGE_PAD}px`, height: `${RULER}px` }">
              <canvas ref="hRuler" class="block" :style="{ marginLeft: `${STAGE_PAD}px` }" />
            </div>
          </div>
        </div>
        <div class="flex min-h-0 flex-1">
          <div
            ref="vRulerView"
            class="shrink-0 overflow-hidden border-r border-slate-700 bg-slate-900"
            :style="{ width: `${RULER}px` }"
            @wheel="onRulerWheel"
          >
            <div :style="{ width: `${RULER}px`, height: `${STAGE_PAD + sheetHeight * scale + STAGE_PAD}px` }">
              <canvas ref="vRuler" class="block" :style="{ marginTop: `${STAGE_PAD}px` }" />
            </div>
          </div>
      <div
        ref="stageEl"
        class="min-w-0 flex-1 overflow-auto bg-slate-900 p-6"
        @scroll="syncRulers"
        @dragover="onCanvasDragOver"
        @dragleave="onCanvasDragLeave"
        @drop="onCanvasDrop"
        @click="tool ? undefined : select({ kind: 'report' })"
      >
        <div :style="{ width: `${canvasWidth * scale}px`, height: `${sheetHeight * scale}px` }">
        <div
          ref="sheetEl"
          class="relative bg-white shadow-lg shadow-black/40"
          :style="{ width: `${canvasWidth}px`, height: `${sheetHeight}px`, transform: `scale(${scale})`, transformOrigin: 'top left' }"
          @click.stop
          @pointermove="trackPointer"
          @pointerleave="pointer = null"
        >
          <div class="bg-slate-200" :style="{ height: `${topMargin}px` }" />
          <div
            v-for="row in bandRows"
            :key="`canvas-${row.index}`"
            class="relative flex"
            :data-band-index="row.index"
            @click="tool ? undefined : select({ kind: 'band', index: row.index })"
          >
            <div class="shrink-0 bg-slate-200" :style="{ width: `${leftMargin}px` }" />
            <div
              data-band-content
              class="relative"
              :class="
                isSelected({ kind: 'band', index: row.index })
                  ? 'bg-blue-50 ring-1 ring-inset ring-blue-400'
                  : dropBand === row.index
                    ? 'bg-white ring-1 ring-inset ring-blue-300'
                    : 'bg-white'
              "
              :style="{ width: `${columnWidth}px`, height: `${stripHeight(row.band.height)}px`, ...gridStyle }"
              @click.stop="onBandClick($event, row.index)"
            >
              <div class="pointer-events-none absolute inset-0 z-0 flex items-center justify-center overflow-hidden">
                <span
                  class="select-none font-medium uppercase tracking-[0.18em]"
                  :class="isSelected({ kind: 'band', index: row.index }) ? 'text-blue-300' : 'text-slate-400/70'"
                  :style="{ fontSize: `${watermarkSize(row.band.height)}px` }"
                >
                  {{ bandLabel(row.band.kind) }}
                </span>
              </div>
              <div
                v-for="(element, index) in row.band.elements"
                :key="`${row.index}-box-${index}`"
                class="absolute overflow-hidden text-black"
                :class="
                  isSelected({ kind: 'element', band: row.index, index })
                    ? 'z-10 ring-2 ring-blue-500'
                    : tableChildSelected(row.index, index)
                      ? 'z-10 ring-1 ring-blue-300'
                      : ''
                "
                :style="{ ...boxStyle(element), ...fillStyle(element), ...borderStyle(element), ...guideStyle(element) }"
                @pointerdown.stop="onElementPointerDown($event, row.index, index)"
                @click.stop
                @dblclick.stop="onDoubleClick(row.index, index)"
              >
                <input
                  v-if="editing && editing.band === row.index && editing.index === index"
                  data-design-text
                  class="h-full w-full bg-white px-1 text-black outline-none"
                  :value="textDraft"
                  @click.stop
                  @pointerdown.stop
                  @blur="commitText"
                  @keydown.enter.prevent="commitText"
                  @keydown.esc.prevent="editing = null"
                  @input="textDraft = ($event.target as HTMLInputElement).value"
                />
                <template v-else-if="element.type === 'table'">
                  <div class="flex h-full flex-col text-[10px] leading-tight">
                    <div class="flex h-4 shrink-0">
                      <button
                        v-for="(column, columnIndex) in element.columns"
                        :key="`bar-${columnIndex}`"
                        type="button"
                        class="truncate border-r border-b border-slate-300 px-1 text-left text-[9px]"
                        :class="
                          isSelected({ kind: 'column', band: row.index, index, column: columnIndex })
                            ? 'bg-blue-200 text-blue-900'
                            : 'bg-slate-100 text-slate-500 hover:bg-slate-200'
                        "
                        :style="{ width: `${column.width}px` }"
                        @pointerdown.stop
                        @click.stop="select({ kind: 'column', band: row.index, index, column: columnIndex })"
                      >
                        Column {{ columnIndex + 1 }}
                      </button>
                    </div>
                    <div class="flex min-h-0 flex-1 flex-col">
                      <div
                        v-for="kind in tableSectionKinds(element)"
                        :key="kind"
                        class="flex min-h-0"
                        :style="{ flex: `${kindRowWeight(element, kind)} 1 0` }"
                      >
                        <div
                          v-for="(column, columnIndex) in element.columns"
                          :key="`${kind}-${columnIndex}`"
                          class="relative h-full overflow-hidden"
                          :class="columnCellClass(row.index, index, columnIndex, kind)"
                          :style="{ width: `${column.width}px`, ...sectionChrome(sectionByKind(column, kind)?.section ?? { kind, style: null, height: 0, elements: [] }) }"
                          @pointerdown.stop
                          @click.stop="selectColumnSection(row.index, index, columnIndex, kind)"
                        >
                          <div
                            v-for="(child, elementIndex) in sectionByKind(column, kind)?.section.elements ?? []"
                            :key="elementIndex"
                            class="absolute overflow-hidden text-black"
                            :class="
                              isSelected({
                                kind: 'cell',
                                band: row.index,
                                index,
                                column: columnIndex,
                                section: sectionByKind(column, kind)?.index ?? -1,
                                element: elementIndex,
                              })
                                ? 'ring-2 ring-inset ring-blue-500'
                                : ''
                            "
                            :style="placedChildStyle(child, column.width, sectionByKind(column, kind)?.section.height ?? 0)"
                            @pointerdown.stop
                            @click.stop="selectCell(row.index, index, columnIndex, kind, elementIndex)"
                          >
                            <img
                              v-if="child.type === 'image' && imagePreview(child)"
                              :src="imagePreview(child)"
                              alt=""
                              class="h-full w-full"
                              :style="imageObjectStyle(child)"
                            />
                            <div v-else-if="child.type === 'image'" class="flex h-full items-center justify-center overflow-hidden px-1 text-center text-[9px] text-slate-500">
                              {{ child.expression || "Image" }}
                            </div>
                            <div v-else class="h-full w-full overflow-hidden" :style="textFrameStyle()">
                              <div :style="textRotateStyle(child)">
                                <span :style="textContentStyle(child)"><template v-for="(piece, pieceIndex) in textPieces(child)" :key="pieceIndex"><span :style="piece.style">{{ piece.text }}</span></template></span>
                              </div>
                            </div>
                          </div>
                        </div>
                      </div>
                    </div>
                  </div>
                </template>
                <template v-else-if="element.type === 'line'">
                  <svg
                    class="pointer-events-none absolute inset-0 overflow-visible"
                    :width="element.element.width"
                    :height="element.element.height"
                  >
                    <line
                      x1="0"
                      y1="0"
                      :x2="element.element.width"
                      :y2="element.element.height"
                      :stroke="borderWidth(element.element) > 0 ? hexColor(borderColor(element.element)) : 'rgba(148,163,184,0.7)'"
                      :stroke-width="borderWidth(element.element) > 0 ? borderWidth(element.element) : 1"
                    />
                  </svg>
                </template>
                <template v-else-if="element.type === 'rectangle'" />
                <template v-else-if="element.type === 'ellipse'">
                  <svg
                    class="pointer-events-none absolute inset-0 overflow-visible"
                    :width="element.element.width"
                    :height="element.element.height"
                  >
                    <ellipse
                      :cx="element.element.width / 2"
                      :cy="element.element.height / 2"
                      :rx="Math.max(element.element.width / 2 - 0.5, 0)"
                      :ry="Math.max(element.element.height / 2 - 0.5, 0)"
                      :stroke="borderWidth(element.element) > 0 ? hexColor(borderColor(element.element)) : 'rgba(148,163,184,0.7)'"
                      :stroke-width="borderWidth(element.element) > 0 ? borderWidth(element.element) : 1"
                      :fill="element.element.mode === 'opaque' ? hexColor(element.element.backcolor ?? { r: 255, g: 255, b: 255 }) : 'none'"
                    />
                  </svg>
                </template>
                <template v-else-if="element.type === 'break'">
                  <div class="flex h-full items-center">
                    <div class="h-px w-full border-t border-dashed border-slate-400" />
                  </div>
                </template>
                <template v-else-if="element.type === 'chart'">
                  <div class="flex h-full flex-col items-center justify-center bg-slate-200 px-1 text-center text-xs text-slate-600">
                    <span>{{ chartKindLabel(element.kind) }}</span>
                    <span v-if="element.title" class="mt-0.5 text-[10px] text-slate-500">{{ element.title }}</span>
                  </div>
                </template>
                <template v-else-if="element.type === 'subreport'">
                  <div class="flex h-full flex-col items-center justify-center bg-slate-100 px-1 text-center text-[10px] leading-tight text-slate-600">
                    <span>Subreport</span>
                    <span class="mt-0.5 max-w-full truncate text-slate-500">{{ subreportPathText(element.expression) || "No file" }}</span>
                  </div>
                </template>
                <template v-else-if="element.type === 'image'">
                  <img
                    v-if="imagePreview(element)"
                    :src="imagePreview(element)"
                    alt=""
                    class="h-full w-full"
                    :style="imageObjectStyle(element)"
                  />
                  <div v-else class="flex h-full items-center justify-center overflow-hidden px-1 text-center text-[10px] leading-tight text-slate-500">
                    {{ element.expression || "Image" }}
                  </div>
                </template>
                <div v-else class="h-full w-full overflow-hidden" :style="textFrameStyle()">
                  <div :style="textRotateStyle(element)">
                    <span :style="textContentStyle(element)"><template v-for="(piece, pieceIndex) in textPieces(element)" :key="pieceIndex"><span :style="piece.style">{{ piece.text }}</span></template></span>
                  </div>
                </div>
              </div>
              <template v-if="selection.kind === 'element' && selection.band === row.index">
                <span
                  v-for="handle in HANDLES"
                  :key="handle"
                  class="absolute z-20 h-2 w-2 -translate-x-1/2 -translate-y-1/2 border border-blue-700 bg-white"
                  @click.stop
                  :style="handlePosition(row.index, handle)"
                  @pointerdown.stop="onHandlePointerDown($event, row.index, selection.index, handle)"
                />
              </template>
            </div>
            <div class="min-w-0 flex-1 bg-slate-200" />
            <div
              class="group absolute right-0 bottom-0 left-0 z-30 h-2 translate-y-1/2 cursor-ns-resize"
              title="Drag to resize the section"
              @pointerdown.stop="onBandPointerDown($event, row.index)"
              @click.stop
            >
              <div
                class="absolute top-1/2 right-0 left-0 h-px -translate-y-1/2 group-hover:bg-blue-500"
                :class="isSelected({ kind: 'band', index: row.index }) ? 'bg-blue-500' : 'bg-slate-400'"
              />
            </div>
          </div>
          <div v-if="pageRemainder > 0" class="flex" :style="{ height: `${pageRemainder}px` }">
            <div class="shrink-0 bg-slate-200" :style="{ width: `${leftMargin}px` }" />
            <div class="min-w-0 flex-1 bg-white" />
            <div class="shrink-0 bg-slate-200" :style="{ width: `${rightMargin}px` }" />
          </div>
          <div class="bg-slate-200" :style="{ height: `${bottomMargin}px` }" />
          <div
            v-if="pastPage"
            class="pointer-events-none absolute right-0 left-0 border-t border-dashed border-amber-500"
            :style="{ top: `${report?.pageHeight ?? 0}px` }"
          />
        </div>
        </div>
      </div>
        </div>
      </div>

      <button
        v-if="!inspectorOpen"
        type="button"
        class="flex w-8 shrink-0 flex-col items-center gap-2 border-l border-slate-800 bg-slate-950 py-2 text-gray-400 hover:bg-slate-900 hover:text-white"
        title="Show palette and properties"
        @click="inspectorOpen = true"
      >
        <Icon name="chevron-left" />
        <span class="text-[10px] font-semibold uppercase tracking-wide [writing-mode:vertical-rl]">Properties</span>
      </button>
      <aside
        v-else
        class="relative flex shrink-0 flex-col border-l border-slate-800"
        :style="{ width: `${inspectorWidth}px` }"
      >
        <div class="flex items-center justify-between border-b border-slate-800 px-3 py-2 text-xs font-semibold uppercase tracking-wide text-gray-400">
          Palette
          <button
            type="button"
            class="rounded p-0.5 text-gray-400 hover:bg-slate-800 hover:text-white"
            title="Hide palette and properties"
            @click="inspectorOpen = false"
          >
            <Icon name="chevron-right" />
          </button>
        </div>
        <div
          class="absolute top-0 -left-1 z-20 h-full w-2 cursor-col-resize hover:bg-blue-500"
          role="separator"
          aria-orientation="vertical"
          title="Resize palette and properties"
          @pointerdown.stop="onPanelResizeDown($event, 'inspector')"
        />
        <div class="grid shrink-0 grid-cols-[repeat(auto-fill,minmax(4.5rem,1fr))] gap-1 p-2">
          <div
            v-for="item in palette"
            :key="item.id"
            role="button"
            tabindex="0"
            class="flex cursor-grab flex-col items-center gap-1 rounded-md border px-1 py-2 text-center text-[10px] leading-tight active:cursor-grabbing"
            :class="
              tool === item.id
                ? 'border-blue-500 bg-blue-600/20 text-white'
                : 'border-slate-800 bg-slate-900 text-gray-200 hover:bg-slate-800'
            "
            :title="item.label"
            draggable="true"
            @dragstart="onToolDragStart($event, item.id)"
            @click="tool = tool === item.id ? null : item.id"
            @keydown.enter.prevent="tool = tool === item.id ? null : item.id"
          >
            <Icon :name="item.icon" size="lg" class="text-blue-300" />
            {{ item.label }}
          </div>
        </div>
        <div class="border-y border-slate-800 px-3 py-2 text-xs font-semibold uppercase tracking-wide text-gray-400">
          Properties
        </div>
        <div class="min-h-0 flex-1 space-y-3 overflow-auto p-3 text-sm">
          <template v-if="selection.kind === 'report'">
            <label class="block text-xs text-gray-400">
              Name
              <input
                :class="inputClass"
                :value="report.name"
                @change="edit(() => { if (report) report.name = ($event.target as HTMLInputElement).value; })"
              />
            </label>
            <label class="block text-xs text-gray-400">
              Page
              <select :class="inputClass" :value="pagePreset" @change="applyPreset(($event.target as HTMLSelectElement).value)">
                <option value="a4">A4</option>
                <option value="letter">Letter</option>
                <option value="custom">Custom</option>
              </select>
            </label>
            <div class="grid grid-cols-2 gap-2">
              <label class="text-xs text-gray-400">
                Width
                <input
                  :class="inputClass"
                  type="number"
                  :value="report.pageWidth"
                  @change="commitNumber(($event.target as HTMLInputElement).value, (value) => { if (report) { report.pageWidth = value; syncColumn(report); } })"
                />
              </label>
              <label class="text-xs text-gray-400">
                Height
                <input
                  :class="inputClass"
                  type="number"
                  :value="report.pageHeight"
                  @change="commitNumber(($event.target as HTMLInputElement).value, (value) => { if (report) report.pageHeight = value; })"
                />
              </label>
              <label class="text-xs text-gray-400">
                Left
                <input
                  :class="inputClass"
                  type="number"
                  :value="report.leftMargin"
                  @change="commitNumber(($event.target as HTMLInputElement).value, (value) => { if (report) { report.leftMargin = value; syncColumn(report); } })"
                />
              </label>
              <label class="text-xs text-gray-400">
                Right
                <input
                  :class="inputClass"
                  type="number"
                  :value="report.rightMargin"
                  @change="commitNumber(($event.target as HTMLInputElement).value, (value) => { if (report) { report.rightMargin = value; syncColumn(report); } })"
                />
              </label>
              <label class="text-xs text-gray-400">
                Top
                <input
                  :class="inputClass"
                  type="number"
                  :value="report.topMargin"
                  @change="commitNumber(($event.target as HTMLInputElement).value, (value) => { if (report) report.topMargin = value; })"
                />
              </label>
              <label class="text-xs text-gray-400">
                Bottom
                <input
                  :class="inputClass"
                  type="number"
                  :value="report.bottomMargin"
                  @change="commitNumber(($event.target as HTMLInputElement).value, (value) => { if (report) report.bottomMargin = value; })"
                />
              </label>
            </div>
            <p class="text-xs text-gray-500">Column width {{ Math.round(report.columnWidth) }}</p>
          </template>

          <template v-else-if="selection.kind === 'fields'">
            <FwbButton size="xs" @click="addFieldFromProperties">Add field</FwbButton>
          </template>

          <template v-else-if="selectedField && (selection.kind === 'field' || selection.kind === 'datasetField')">
            <label class="block text-xs text-gray-400">
              Name
              <input
                :class="inputClass"
                :value="selectedField.name"
                @change="edit(() => { if (selectedField) selectedField.name = ($event.target as HTMLInputElement).value; })"
              />
            </label>
            <label class="block text-xs text-gray-400">
              Class
              <select
                :class="inputClass"
                :value="selectedField.className"
                @change="edit(() => { if (selectedField) selectedField.className = ($event.target as HTMLSelectElement).value; })"
              >
                <option value="java.lang.String">String</option>
                <option value="java.lang.Integer">Integer</option>
              </select>
            </label>
            <label class="block text-xs text-gray-400">
              JSON path
              <input
                :class="inputClass"
                :value="selectedField.jsonPath ?? ''"
                @change="edit(() => { if (selectedField) selectedField.jsonPath = ($event.target as HTMLInputElement).value || null; })"
              />
            </label>
            <FwbButton
              size="xs"
              color="red"
              @click="edit(() => removeSelectedField())"
            >
              Remove field
            </FwbButton>
          </template>

          <template v-else-if="selection.kind === 'dataset'">
            <FwbButton size="xs" @click="addDatasetFieldFromProperties">Add field</FwbButton>
          </template>

          <template v-else-if="selectedBand && bandMonitor">
            <p class="text-sm font-medium text-gray-100">{{ bandLabel(selectedBand.kind) }}</p>
            <label class="block text-xs text-gray-400">
              Height
              <input
                :class="inputClass"
                type="number"
                min="0"
                :value="Math.round(selectedBand.height)"
                @change="setBandHeight(($event.target as HTMLInputElement).value)"
              />
            </label>
            <p class="text-xs text-gray-500">Split type is Stretch. Prevent and Immediate are outside this subset.</p>
            <p class="text-xs text-gray-400">
              Starts at {{ Math.round(bandMonitor.start) }} and ends at {{ Math.round(bandMonitor.end) }} on the page.
              The page edge is {{ Math.round(bandMonitor.pageEdge) }}.
            </p>
            <p v-if="bandMonitor.past" class="text-xs text-amber-400">This band starts past the page edge.</p>
            <p v-else-if="bandMonitor.crosses" class="text-xs text-amber-400">This band crosses the dashed page edge.</p>
            <p v-else-if="bandMonitor.stackOverflows" class="text-xs text-amber-400">
              The bands use {{ Math.round(bandMonitor.stack) }} points. The page content area is {{ Math.round(bandMonitor.contentLimit) }}.
            </p>
            <p v-else class="text-xs text-gray-500">
              The bands fit in the page content area of {{ Math.round(bandMonitor.contentLimit) }} points.
            </p>
            <p v-if="bandMonitor.floor > 0" class="text-xs text-gray-500">
              Height stays at least {{ Math.round(bandMonitor.floor) }} so the elements still fit.
            </p>
            <FwbButton size="xs" color="red" @click="removeSelected">Remove band</FwbButton>
          </template>

          <template v-else-if="selection.kind === 'tableSection' && selectedTable && selectedTableSection">
            <p class="text-sm font-medium text-gray-100">{{ sectionLabel(selection.section) }}</p>
            <label class="block text-xs text-gray-400">
              Height
              <input
                :class="inputClass"
                type="number"
                min="0"
                :value="selectedTableSection.height"
                @change="commitNumber(($event.target as HTMLInputElement).value, (value) => { if (selectedTable && selection.kind === 'tableSection') setSectionsHeight(selectedTable, selection.section, value); })"
              />
            </label>
            <label class="block text-xs text-gray-400">
              Style
              <input
                :class="inputClass"
                :value="selectedTableSection.style ?? ''"
                @change="edit(() => { if (selectedTable && selection.kind === 'tableSection') setSectionsStyle(selectedTable, selection.section, ($event.target as HTMLInputElement).value || null); })"
              />
            </label>
            <p class="text-xs text-gray-500">Height and style apply to this section in every column.</p>
          </template>

          <template v-else-if="selection.kind === 'column' && selectedColumn">
            <p class="text-sm font-medium text-gray-100">Column {{ selection.column + 1 }}</p>
            <label class="block text-xs text-gray-400">
              Width
              <input
                :class="inputClass"
                type="number"
                min="1"
                :value="selectedColumn.width"
                @change="commitNumber(($event.target as HTMLInputElement).value, (value) => { if (selectedColumn && selectedTable) { selectedColumn.width = Math.max(1, value); syncTableBox(selectedTable); } })"
              />
            </label>
            <FwbButton v-if="selectedTable.columns.length > 1" size="xs" color="red" @click="removeSelected">Remove column</FwbButton>
          </template>

          <template v-else-if="selection.kind === 'columnSection' && selectedColumnSection">
            <p class="text-sm font-medium text-gray-100">{{ sectionLabel(selectedColumnSection.kind) }}</p>
            <label class="block text-xs text-gray-400">
              Height
              <input
                :class="inputClass"
                type="number"
                min="0"
                :value="selectedColumnSection.height"
                @change="commitNumber(($event.target as HTMLInputElement).value, (value) => { if (selectedColumnSection && selectedTable) { selectedColumnSection.height = Math.max(0, value); syncTableBox(selectedTable); } })"
              />
            </label>
            <label class="block text-xs text-gray-400">
              Style
              <input
                :class="inputClass"
                :value="selectedColumnSection.style ?? ''"
                @change="edit(() => { if (selectedColumnSection) selectedColumnSection.style = ($event.target as HTMLInputElement).value || null; })"
              />
            </label>
          </template>

          <template v-else-if="editedElement && (selection.kind === 'element' || selection.kind === 'cell')">
            <div class="grid grid-cols-2 gap-2">
              <label class="text-xs text-gray-400">
                X
                <input :class="inputClass" type="number" :value="editedElement.element.x" @change="commitNumber(($event.target as HTMLInputElement).value, (value) => { if (editedElement) { editedElement.element.x = value; clampEdited(editedElement.element); } })" />
              </label>
              <label class="text-xs text-gray-400">
                Y
                <input :class="inputClass" type="number" :value="editedElement.element.y" @change="commitNumber(($event.target as HTMLInputElement).value, (value) => { if (editedElement) { editedElement.element.y = value; clampEdited(editedElement.element); } })" />
              </label>
              <label class="text-xs text-gray-400">
                Width
                <input :class="inputClass" type="number" :value="editedElement.element.width" @change="commitNumber(($event.target as HTMLInputElement).value, (value) => { if (editedElement) { editedElement.element.width = value; clampEdited(editedElement.element); } })" />
              </label>
              <label class="text-xs text-gray-400">
                Height
                <input :class="inputClass" type="number" :value="editedElement.element.height" @change="commitNumber(($event.target as HTMLInputElement).value, (value) => { if (editedElement) { editedElement.element.height = value; clampEdited(editedElement.element); } })" />
              </label>
            </div>

            <template v-if="editedElement.type === 'chart'">
              <p class="text-xs text-gray-400">
                <template v-if="plotsChart(editedElement.kind)">Preview plots this chart.</template>
                <template v-else>Preview keeps the placeholder for {{ editedElement.kind }}.</template>
              </p>
              <label class="block text-xs text-gray-400">
                Kind
                <select :class="inputClass" :value="editedElement.kind" @change="edit(() => { if (editedElement?.type === 'chart') editedElement.kind = ($event.target as HTMLSelectElement).value; })">
                  <option v-if="!plotsChart(editedElement.kind)" :value="editedElement.kind">{{ editedElement.kind }}</option>
                  <option v-for="kind in CHART_KINDS" :key="kind.id" :value="kind.id">{{ kind.label }}</option>
                </select>
              </label>
              <label class="block text-xs text-gray-400">
                Title
                <input :class="inputClass" :value="editedElement.title ?? ''" placeholder="Chart title" @change="edit(() => { if (editedElement?.type === 'chart') editedElement.title = chartExpression(($event.target as HTMLInputElement).value) || null; })" />
              </label>
              <label class="block text-xs text-gray-400">
                Subtitle
                <input :class="inputClass" :value="editedElement.subtitle ?? ''" @change="edit(() => { if (editedElement?.type === 'chart') editedElement.subtitle = chartExpression(($event.target as HTMLInputElement).value) || null; })" />
              </label>
              <label class="flex items-center gap-2 text-xs text-gray-300">
                <input
                  type="checkbox"
                  :checked="editedElement.showLegend !== false"
                  @change="edit(() => { if (editedElement?.type === 'chart') editedElement.showLegend = ($event.target as HTMLInputElement).checked; })"
                />
                Show legend
              </label>
              <label class="block text-xs text-gray-400">
                Evaluation
                <select :class="inputClass" :value="editedElement.evaluationTime ?? ''" @change="edit(() => { if (editedElement?.type === 'chart') editedElement.evaluationTime = ($event.target as HTMLSelectElement).value || null; })">
                  <option value="">Now</option>
                  <option value="Now">Now</option>
                  <option value="Report">Report</option>
                  <option value="Page">Page</option>
                  <option value="Column">Column</option>
                  <option value="Band">Band</option>
                </select>
              </label>
              <label class="block text-xs text-gray-400">
                Data array
                <input
                  :class="inputClass"
                  :value="editedElement.dataSource ?? ''"
                  placeholder="chartPoints"
                  @change="edit(() => { if (report && editedElement?.type === 'chart') bindChartSource(report, editedElement, ($event.target as HTMLInputElement).value); })"
                />
              </label>
              <p class="text-xs text-gray-500">Each object in that JSON array is one point. Leave it empty to read the main record once.</p>
              <div v-for="(series, index) in editedElement.series ?? []" :key="index" class="space-y-2 rounded-md border border-slate-800 p-2">
                <div class="flex items-center justify-between text-xs text-gray-400">
                  Series {{ index + 1 }}
                  <button type="button" class="text-red-400 hover:text-red-300" @click="edit(() => { if (editedElement?.type === 'chart') editedElement.series?.splice(index, 1); })">Remove</button>
                </div>
                <label class="block text-xs text-gray-400">
                  Name
                  <input :class="inputClass" :value="series.seriesExpression" @change="edit(() => { if (editedElement?.type === 'chart') setChartExpression(editedElement, series, 'seriesExpression', ($event.target as HTMLInputElement).value); })" />
                </label>
                <label class="block text-xs text-gray-400">
                  Category
                  <input :class="inputClass" :value="series.categoryExpression" @change="edit(() => { if (editedElement?.type === 'chart') setChartExpression(editedElement, series, 'categoryExpression', ($event.target as HTMLInputElement).value); })" />
                </label>
                <label class="block text-xs text-gray-400">
                  Use
                  <select :class="inputClass" @change="edit(() => { const value = ($event.target as HTMLSelectElement).value; if (value && report && editedElement?.type === 'chart') setChartExpression(editedElement, series, 'categoryExpression', value); })">
                    <option value="">Choose</option>
                    <option v-for="choice in chartChoices(report, editedElement, series.categoryExpression)" :key="choice" :value="choice">{{ choice }}</option>
                  </select>
                </label>
                <label class="block text-xs text-gray-400">
                  Value
                  <input :class="inputClass" :value="series.valueExpression" @change="edit(() => { if (editedElement?.type === 'chart') setChartExpression(editedElement, series, 'valueExpression', ($event.target as HTMLInputElement).value); })" />
                </label>
                <label class="block text-xs text-gray-400">
                  Use
                  <select :class="inputClass" @change="edit(() => { const value = ($event.target as HTMLSelectElement).value; if (value && report && editedElement?.type === 'chart') setChartExpression(editedElement, series, 'valueExpression', value); })">
                    <option value="">Choose</option>
                    <option v-for="choice in chartChoices(report, editedElement, series.valueExpression)" :key="choice" :value="choice">{{ choice }}</option>
                  </select>
                </label>
              </div>
              <FwbButton size="xs" @click="edit(() => { if (editedElement?.type === 'chart') addChartSeries(editedElement); })">Add series</FwbButton>
            </template>

            <template v-if="editedElement.type === 'subreport'">
              <label class="block text-xs text-gray-400">
                Report file
                <input
                  :class="inputClass"
                  :value="editedElement.expression"
                  placeholder='"section.jrxml"'
                  @change="edit(() => { if (editedElement?.type === 'subreport') editedElement.expression = chartExpression(($event.target as HTMLInputElement).value); })"
                />
              </label>
              <div class="flex flex-wrap gap-2">
                <FwbButton size="xs" @click="browseSubreport">Browse</FwbButton>
                <FwbButton size="xs" @click="createSubreport">New</FwbButton>
                <FwbButton size="xs" :disabled="!editedElement.expression" @click="openSubreport(editedElement.expression)">Open</FwbButton>
              </div>
              <label class="block text-xs text-gray-400">
                Data path
                <input
                  :class="inputClass"
                  :value="editedElement.dataSource ?? ''"
                  placeholder="orders"
                  @change="setSubreportSource(($event.target as HTMLInputElement).value)"
                />
              </label>
              <p class="text-xs text-gray-500">
                Leave the path empty to fill the other file once from the current JSON object. A path such as customer.orders repeats its detail band, one object per element.
              </p>
              <FwbButton size="xs" color="alternative" @click="setSubreportSource('')">Use this object</FwbButton>
              <div class="space-y-2 rounded-md border border-slate-800 p-2">
                <p class="text-xs text-gray-400">JSON file</p>
                <p v-if="jsonError" class="text-xs text-amber-400">{{ jsonError }}</p>
                <p v-else-if="jsonDocument === null" class="text-xs text-gray-500">
                  Choose a JSON file in Dataset.
                  <button type="button" class="text-blue-400 hover:text-blue-300" @click="emit('openDataset')">
                    Open Dataset
                  </button>
                </p>
                <JsonTree
                  v-else
                  :value="jsonDocument"
                  :selected="editedElement.dataSource ?? ''"
                  @select="setSubreportSource"
                />
              </div>
            </template>

            <template v-if="editedElement.type === 'image'">
              <label class="block text-xs text-gray-400">
                Image expression
                <input
                  :class="inputClass"
                  :value="editedElement.expression"
                  placeholder='"logo.png" or $F{logo}'
                  @change="edit(() => { if (editedElement?.type === 'image') editedElement.expression = commitImageExpression(($event.target as HTMLInputElement).value); })"
                />
              </label>
              <label class="block text-xs text-gray-400">
                Expression class
                <input
                  :class="inputClass"
                  :value="editedElement.expressionClass ?? ''"
                  placeholder="java.lang.String"
                  @change="edit(() => { if (editedElement?.type === 'image') editedElement.expressionClass = ($event.target as HTMLInputElement).value.trim() || null; })"
                />
              </label>
              <label class="block text-xs text-gray-400">
                Scale
                <select :class="inputClass" :value="editedElement.scaleImage ?? ''" @change="edit(() => { if (editedElement?.type === 'image') editedElement.scaleImage = (($event.target as HTMLSelectElement).value || null) as typeof editedElement.scaleImage; })">
                  <option value="">Retain shape</option>
                  <option value="clip">Clip</option>
                  <option value="fillFrame">Fill frame</option>
                  <option value="retainShape">Retain shape</option>
                  <option value="realHeight">Real height</option>
                  <option value="realSize">Real size</option>
                </select>
              </label>
              <div class="grid grid-cols-2 gap-2">
                <label class="text-xs text-gray-400">
                  Horizontal
                  <select :class="inputClass" :value="editedElement.hAlign ?? ''" @change="edit(() => { if (editedElement?.type === 'image') editedElement.hAlign = (($event.target as HTMLSelectElement).value || null) as typeof editedElement.hAlign; })">
                    <option value="">Left</option>
                    <option value="left">Left</option>
                    <option value="center">Center</option>
                    <option value="right">Right</option>
                  </select>
                </label>
                <label class="text-xs text-gray-400">
                  Vertical
                  <select :class="inputClass" :value="editedElement.vAlign ?? ''" @change="edit(() => { if (editedElement?.type === 'image') editedElement.vAlign = (($event.target as HTMLSelectElement).value || null) as typeof editedElement.vAlign; })">
                    <option value="">Top</option>
                    <option value="top">Top</option>
                    <option value="middle">Middle</option>
                    <option value="bottom">Bottom</option>
                  </select>
                </label>
              </div>
              <label class="block text-xs text-gray-400">
                On error
                <select :class="inputClass" :value="editedElement.onErrorType ?? ''" @change="edit(() => { if (editedElement?.type === 'image') editedElement.onErrorType = (($event.target as HTMLSelectElement).value || null) as typeof editedElement.onErrorType; })">
                  <option value="">Error</option>
                  <option value="error">Error</option>
                  <option value="blank">Blank</option>
                  <option value="icon">Icon</option>
                </select>
              </label>
              <label class="block text-xs text-gray-400">
                Evaluation
                <select :class="inputClass" :value="editedElement.evaluationTime ?? ''" @change="edit(() => { if (editedElement?.type === 'image') { editedElement.evaluationTime = ($event.target as HTMLSelectElement).value || null; if (editedElement.evaluationTime !== 'Group') editedElement.evaluationGroup = null; } })">
                  <option value="">Now</option>
                  <option value="Now">Now</option>
                  <option value="Report">Report</option>
                  <option value="Page">Page</option>
                  <option value="Column">Column</option>
                  <option value="Group">Group</option>
                  <option value="Band">Band</option>
                  <option value="Auto">Auto</option>
                </select>
              </label>
              <label v-if="editedElement.evaluationTime === 'Group'" class="block text-xs text-gray-400">
                Evaluation group
                <input :class="inputClass" :value="editedElement.evaluationGroup ?? ''" @change="edit(() => { if (editedElement?.type === 'image') editedElement.evaluationGroup = ($event.target as HTMLInputElement).value || null; })" />
              </label>
              <label class="flex items-center gap-2 text-xs text-gray-300">
                <input
                  type="checkbox"
                  :checked="!!editedElement.lazy"
                  @change="edit(() => { if (editedElement?.type === 'image') editedElement.lazy = ($event.target as HTMLInputElement).checked; })"
                />
                Lazy
              </label>
              <label class="block text-xs text-gray-400">
                Cache
                <select :class="inputClass" :value="editedElement.usingCache === true ? 'true' : editedElement.usingCache === false ? 'false' : ''" @change="edit(() => { if (editedElement?.type === 'image') { const value = ($event.target as HTMLSelectElement).value; editedElement.usingCache = value === '' ? null : value === 'true'; } })">
                  <option value="">Automatic</option>
                  <option value="true">Yes</option>
                  <option value="false">No</option>
                </select>
              </label>
              <label class="block text-xs text-gray-400">
                Rotation
                <select :class="inputClass" :value="editedElement.rotation ?? ''" @change="edit(() => { if (editedElement?.type === 'image') editedElement.rotation = (($event.target as HTMLSelectElement).value || null) as typeof editedElement.rotation; })">
                  <option value="">None</option>
                  <option value="left">Left</option>
                  <option value="right">Right</option>
                  <option value="upsideDown">Upside down</option>
                </select>
              </label>
            </template>

            <template v-if="editedElement.type === 'staticText' || editedElement.type === 'textField'">
              <label v-if="editedElement.type === 'staticText'" class="block text-xs text-gray-400">
                Text
                <input :class="inputClass" :value="editedElement.text" @change="edit(() => { if (editedElement?.type === 'staticText') editedElement.text = ($event.target as HTMLInputElement).value; })" />
              </label>
              <label v-else class="block text-xs text-gray-400">
                Expression
                <input
                  :class="inputClass"
                  :value="editedElement.expression"
                  @change="edit(() => { if (editedElement?.type === 'textField') editedElement.expression = ($event.target as HTMLInputElement).value; })"
                />
              </label>
              <label v-if="editedElement.type === 'textField'" class="block text-xs text-gray-400">
                Use
                <select
                  :class="inputClass"
                  @change="edit(() => { const value = ($event.target as HTMLSelectElement).value; if (value && editedElement?.type === 'textField') editedElement.expression = value; })"
                >
                  <option value="">Choose</option>
                  <option v-for="choice in choicesFor(editedElement)" :key="choice" :value="choice">{{ choice }}</option>
                </select>
              </label>
              <label class="block text-xs text-gray-400">
                Font size
                <input :class="inputClass" type="number" :value="editedElement.textStyle.fontSize ?? ''" @change="commitNumber(($event.target as HTMLInputElement).value, (value) => { if (editedElement && (editedElement.type === 'staticText' || editedElement.type === 'textField')) editedElement.textStyle.fontSize = value; })" />
              </label>
              <label class="flex items-center gap-2 text-xs text-gray-300">
                <input
                  type="checkbox"
                  :checked="editedElement.textStyle.bold"
                  @change="edit(() => { if (editedElement && (editedElement.type === 'staticText' || editedElement.type === 'textField')) editedElement.textStyle.bold = ($event.target as HTMLInputElement).checked; })"
                />
                Bold
              </label>
              <label class="flex items-center gap-2 text-xs text-gray-300">
                <input
                  type="checkbox"
                  :checked="!!editedElement.textStyle.italic"
                  @change="edit(() => { if (editedElement && (editedElement.type === 'staticText' || editedElement.type === 'textField')) editedElement.textStyle.italic = ($event.target as HTMLInputElement).checked; })"
                />
                Italic
              </label>
              <label class="flex items-center gap-2 text-xs text-gray-300">
                <input
                  type="checkbox"
                  :checked="!!editedElement.textStyle.underline"
                  @change="edit(() => { if (editedElement && (editedElement.type === 'staticText' || editedElement.type === 'textField')) editedElement.textStyle.underline = ($event.target as HTMLInputElement).checked; })"
                />
                Underline
              </label>
              <label class="flex items-center gap-2 text-xs text-gray-300">
                <input
                  type="checkbox"
                  :checked="!!editedElement.textStyle.strikeThrough"
                  @change="edit(() => { if (editedElement && (editedElement.type === 'staticText' || editedElement.type === 'textField')) editedElement.textStyle.strikeThrough = ($event.target as HTMLInputElement).checked; })"
                />
                Strike
              </label>
              <label class="block text-xs text-gray-400">
                Markup
                <select :class="inputClass" :value="editedElement.textStyle.markup ?? 'none'" @change="edit(() => { if (editedElement && (editedElement.type === 'staticText' || editedElement.type === 'textField')) editedElement.textStyle.markup = ($event.target as HTMLSelectElement).value as 'none' | 'styled' | 'html'; })">
                  <option value="none">None</option>
                  <option value="styled">Styled</option>
                  <option value="html">HTML</option>
                </select>
              </label>
              <label class="flex items-center gap-2 text-xs text-gray-300">
                <input
                  type="checkbox"
                  :checked="!!editedElement.stretch"
                  @change="edit(() => { if (editedElement && (editedElement.type === 'staticText' || editedElement.type === 'textField')) editedElement.stretch = ($event.target as HTMLInputElement).checked; })"
                />
                Stretch
              </label>
              <div class="grid grid-cols-2 gap-2">
                <label class="text-xs text-gray-400">
                  Align
                  <select :class="inputClass" :value="editedElement.textStyle.textAlignment ?? ''" @change="edit(() => { if (editedElement && (editedElement.type === 'staticText' || editedElement.type === 'textField')) editedElement.textStyle.textAlignment = (($event.target as HTMLSelectElement).value || null) as typeof editedElement.textStyle.textAlignment; })">
                    <option value="">Default</option>
                    <option value="left">Left</option>
                    <option value="center">Center</option>
                    <option value="right">Right</option>
                    <option value="justified">Justified</option>
                  </select>
                </label>
                <label class="text-xs text-gray-400">
                  Vertical
                  <select :class="inputClass" :value="editedElement.textStyle.verticalAlignment ?? ''" @change="edit(() => { if (editedElement && (editedElement.type === 'staticText' || editedElement.type === 'textField')) editedElement.textStyle.verticalAlignment = (($event.target as HTMLSelectElement).value || null) as typeof editedElement.textStyle.verticalAlignment; })">
                    <option value="">Default</option>
                    <option value="top">Top</option>
                    <option value="middle">Middle</option>
                    <option value="bottom">Bottom</option>
                  </select>
                </label>
                <label class="text-xs text-gray-400">
                  Spacing
                  <select :class="inputClass" :value="editedElement.textStyle.lineSpacing ?? ''" @change="edit(() => { if (editedElement && (editedElement.type === 'staticText' || editedElement.type === 'textField')) editedElement.textStyle.lineSpacing = (($event.target as HTMLSelectElement).value || null) as typeof editedElement.textStyle.lineSpacing; })">
                    <option value="">Single</option>
                    <option value="oneAndHalf">1.5</option>
                    <option value="double">Double</option>
                    <option value="proportional">Proportional</option>
                    <option value="atLeast">At least</option>
                    <option value="fixed">Fixed</option>
                  </select>
                </label>
                <label class="text-xs text-gray-400">
                  Rotation
                  <select :class="inputClass" :value="editedElement.textStyle.rotation ?? ''" @change="edit(() => { if (editedElement && (editedElement.type === 'staticText' || editedElement.type === 'textField')) editedElement.textStyle.rotation = (($event.target as HTMLSelectElement).value || null) as typeof editedElement.textStyle.rotation; })">
                    <option value="">None</option>
                    <option value="left">Left</option>
                    <option value="right">Right</option>
                    <option value="upsideDown">Upside down</option>
                  </select>
                </label>
              </div>
              <div class="grid grid-cols-2 gap-2">
                <label class="text-xs text-gray-400">
                  First indent
                  <input :class="inputClass" type="number" :value="editedElement.textStyle.firstLineIndent ?? ''" @change="commitNumber(($event.target as HTMLInputElement).value, (value) => { if (editedElement && (editedElement.type === 'staticText' || editedElement.type === 'textField')) editedElement.textStyle.firstLineIndent = value; })" />
                </label>
                <label class="text-xs text-gray-400">
                  Space before
                  <input :class="inputClass" type="number" :value="editedElement.textStyle.spacingBefore ?? ''" @change="commitNumber(($event.target as HTMLInputElement).value, (value) => { if (editedElement && (editedElement.type === 'staticText' || editedElement.type === 'textField')) editedElement.textStyle.spacingBefore = value; })" />
                </label>
                <label class="text-xs text-gray-400">
                  Tab width
                  <input :class="inputClass" type="number" :value="editedElement.textStyle.tabStopWidth ?? ''" @change="commitNumber(($event.target as HTMLInputElement).value, (value) => { if (editedElement && (editedElement.type === 'staticText' || editedElement.type === 'textField')) editedElement.textStyle.tabStopWidth = value; })" />
                </label>
              </div>
            </template>

            <div class="grid grid-cols-2 gap-2">
              <label class="text-xs text-gray-400">
                Border
                <input
                  :class="inputClass"
                  type="number"
                  min="0"
                  step="0.5"
                  :value="borderWidth(editedElement.element)"
                  @change="commitNumber(($event.target as HTMLInputElement).value, (value) => { if (editedElement) setBorderWidth(editedElement.element, value); })"
                />
              </label>
              <label class="text-xs text-gray-400">
                Border color
                <input
                  :class="inputClass"
                  type="color"
                  :value="hexColor(borderColor(editedElement.element))"
                  @change="edit(() => { if (editedElement) setBorderColor(editedElement.element, colorFromHex(($event.target as HTMLInputElement).value)); })"
                />
              </label>
            </div>

            <FwbButton v-if="editedElement.type === 'table' && report" size="xs" @click="edit(() => { if (report && editedElement?.type === 'table') addColumn(report, editedElement); })">
              Add column
            </FwbButton>

            <FwbButton size="xs" color="red" @click="removeSelected">
              Delete
            </FwbButton>
          </template>
        </div>
      </aside>
    </div>
  </section>
</template>
